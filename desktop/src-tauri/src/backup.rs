//! Two matching, locally verified artifacts. Does not claim off-device durability.
use crate::{excel, store, drive_backup, drive_api};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::{Path,PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};
use chrono::{DateTime, Datelike, Utc};
use tauri::Manager;

#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct BackupStatus {
    pub last_backup_unix_ms: Option<u128>,
    pub local_only: bool,
    pub directory: String,
    pub external_directory: Option<String>,
    pub last_external_backup_unix_ms: Option<u128>,
}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct BackupResult {
    pub sqlite_path: String,
    pub excel_path: String,
    pub manifest_path: String,
    pub external_copy_ok: bool,
    pub external_copy_error: Option<String>,
    pub drive_copy_ok: bool,
    pub drive_copy_error: Option<String>,
    pub cloud_copy_ok: bool,
    pub cloud_copy_error: Option<String>,
}
fn err(e:impl std::fmt::Display)->String {format!("No se pudo crear o verificar el respaldo: {e}")}
fn now_ms()->Result<u128,String> {Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_millis())}
fn backup_dir(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let dir=app.path().app_local_data_dir().map_err(err)?.join("backups");
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir)
}
fn sha(path:&Path)->Result<String,String>{
    use std::io::Read;
    let mut file=fs::File::open(path).map_err(err)?;
    let mut digest=Sha256::new();
    let mut buf=[0u8;128*1024];
    loop {
        let n=file.read(&mut buf).map_err(err)?;
        if n==0{break;}
        digest.update(&buf[..n]);
    }
    Ok(format!("{:x}",digest.finalize()))
}
fn flush_verified_file(path:&Path)->Result<(),String>{
    // On Windows File::open grants read-only access; FlushFileBuffers requires
    // a writable handle, otherwise it fails with os error 5 (Access denied).
    fs::OpenOptions::new().write(true).open(path).map_err(err)?
        .sync_all().map_err(err)
}
/// Publish one new artifact with an atomic no-overwrite reservation. An
/// existence check followed by rename/copy can lose a race to another backup
/// writer, especially on Unix where rename replaces an existing destination.
/// Track ownership immediately, so failure cleanup never deletes foreign files.
/// The copier is injected only for deterministic I/O-failure tests.
/// The production copier is std::io::copy; ownership is acquired BEFORE I/O,
/// allowing the calling transaction to remove every partially written file.
fn publish_new_file_with<F>(
    source:&Path,destination:&Path,owned:&mut Vec<PathBuf>,copy:F
)->Result<(),String>
where F:FnOnce(&mut fs::File,&mut fs::File)->std::io::Result<u64>{
    let mut out=fs::OpenOptions::new().write(true).create_new(true)
        .open(destination).map_err(err)?;
    owned.push(destination.to_path_buf());
    let mut input=fs::File::open(source).map_err(err)?;
    copy(&mut input,&mut out).map_err(err)?;
    out.sync_all().map_err(err)
}
fn publish_new_file(source:&Path,destination:&Path,owned:&mut Vec<PathBuf>)->Result<(),String>{
    publish_new_file_with(source,destination,owned,std::io::copy)
}
fn rollback_owned_files(owned:Vec<PathBuf>){
    // Only paths created by this specific operation are eligible for cleanup.
    // Never remove an existing destination that another process owns.
    for path in owned { let _=fs::remove_file(path); }
}
// Bounded verifier for untrusted external backup triplets. Streaming SHA-256
// does not protect the later manifest/XLSX parser from oversized allocations.
const MAX_VERIFIED_SQLITE_BYTES:u64=512_000_000;
const MAX_VERIFIED_EXCEL_BYTES:u64=25_000_000;
const MAX_VERIFIED_MANIFEST_BYTES:u64=16_384;
fn bounded_backup_file(path:&Path,max:u64)->Result<(),String>{
    let metadata=fs::symlink_metadata(path).map_err(err)?;
    if !metadata.is_file()||metadata.file_type().is_symlink()||metadata.len()==0||metadata.len()>max{
        return Err("Respaldo inválido: archivo ausente, enlace simbólico o tamaño fuera de límites".into());
    }
    Ok(())
}
fn verify_db(conn:&Connection)->Result<(),String>{
    let v:String=conn.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(err)?;
    if v!="ok"{return Err(format!("SQLite integrity_check: {v}"));}
    let fk:i64=conn.query_row("SELECT count(*) FROM pragma_foreign_key_check",[],|r|r.get(0)).map_err(err)?;
    if fk!=0{return Err("El respaldo tiene relaciones huérfanas".into());}
    Ok(())
}
fn snapshot(src:&Connection, dst:&Path)->Result<(),String>{
    let mut conn=Connection::open(dst).map_err(err)?;
    let op=Backup::new(src,&mut conn).map_err(err)?;
    op.run_to_completion(100,Duration::from_millis(50),None).map_err(err)?;
    drop(op);
    verify_db(&conn)
}
pub(crate) fn backup_pair(src:&Connection, dest:&Path, id:&str)->Result<BackupResult,String>{
    fs::create_dir_all(dest).map_err(err)?;
    let db=dest.join(format!("AutoTrace_{id}.sqlite3"));
    let excel=dest.join(format!("AutoTrace_{id}.xlsx"));
    let manifest=dest.join(format!("AutoTrace_{id}.manifest.json"));
    if [db.as_path(),excel.as_path(),manifest.as_path()].iter().any(|p|p.exists()) {return Err("Ya existe un respaldo con ese identificador".into());}
    let staging=dest.join(format!(".autotrace-{id}.partial"));
    fs::create_dir(&staging).map_err(err)?;
    let mut owned=Vec::new();
    let result=(||{
        let temp_db=staging.join("data.sqlite3");
        snapshot(src,&temp_db)?;
        let snap=Connection::open(&temp_db).map_err(err)?;
        verify_db(&snap)?;
        let s=store::read(&snap)?;
        let temp_xlsx=staging.join("data.xlsx");
        fs::write(&temp_xlsx,excel::to_xlsx(&s.data)?).map_err(err)?;
        let bytes=fs::read(&temp_xlsx).map_err(err)?;
        let restored=excel::from_xlsx(&bytes)?;
        if excel::canonical_bytes(&restored)?!=excel::canonical_bytes(&s.data)?{return Err("Excel no reproduce el snapshot SQLite".into());}
        drop(snap); // Windows no permite mover un .sqlite3 mientras la conexión lo mantiene abierto.
        let metadata=serde_json::json!({"kind":"autotrace-dual-backup","version":1,"snapshotId":id,"revision":s.revision,"clients":s.data.clients_count(),"vehicles":s.data.vehicles_count(),"orders":s.data.orders_count(),"sqliteSha256":sha(&temp_db)?,"excelSha256":sha(&temp_xlsx)?});
        let temp_manifest=staging.join("manifest.json");
        fs::write(&temp_manifest,serde_json::to_vec_pretty(&metadata).map_err(err)?).map_err(err)?;
        // Ensure file contents reach the OS before publishing the completion marker.
        // A power-loss test on real hardware remains a separate acceptance gate.
        for file in [&temp_db,&temp_xlsx,&temp_manifest]{flush_verified_file(file)?;}
        // Manifest acts as completion marker and is always promoted last.
        publish_new_file(&temp_db,&db,&mut owned)?;
        publish_new_file(&temp_xlsx,&excel,&mut owned)?;
        publish_new_file(&temp_manifest,&manifest,&mut owned)?;
        if sha(&db)?!=metadata["sqliteSha256"].as_str().unwrap_or("") || sha(&excel)?!=metadata["excelSha256"].as_str().unwrap_or("") {return Err("Checksum no coincide después de copiar".into());}
        Ok(BackupResult{sqlite_path:db.display().to_string(),excel_path:excel.display().to_string(),manifest_path:manifest.display().to_string(),external_copy_ok:false,external_copy_error:None,drive_copy_ok:false,drive_copy_error:None,cloud_copy_ok:false,cloud_copy_error:None})
    })();
    let _=fs::remove_dir_all(staging);
    if result.is_err(){rollback_owned_files(owned);}
    result
}
pub(crate) fn verify_pair_marker(path:&Path)->Result<(),String>{
    bounded_backup_file(path,MAX_VERIFIED_MANIFEST_BYTES)?;
    let manifest:serde_json::Value=serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
    if manifest["kind"]!="autotrace-dual-backup" || manifest["version"]!=1 {
        return Err("Manifiesto de respaldo no reconocido".into());
    }
    let id=manifest["snapshotId"].as_str().ok_or("Manifiesto sin ID")?;
    if id.is_empty()||id.len()>120||!id.chars().all(|c|c.is_ascii_alphanumeric()||c=='-') {
        return Err("ID de respaldo inválido".into());
    }
    let dir=path.parent().ok_or("Carpeta de respaldo inválida")?;
    let expected=format!("AutoTrace_{id}.manifest.json");
    if path.file_name().and_then(|s|s.to_str())!=Some(expected.as_str()) {
        return Err("La identidad del manifiesto y el nombre del archivo no coinciden".into());
    }
    let database=dir.join(format!("AutoTrace_{id}.sqlite3"));
    let excel_path=dir.join(format!("AutoTrace_{id}.xlsx"));
    bounded_backup_file(&database,MAX_VERIFIED_SQLITE_BYTES)?;
    bounded_backup_file(&excel_path,MAX_VERIFIED_EXCEL_BYTES)?;
    if sha(&database)?!=manifest["sqliteSha256"].as_str().unwrap_or("") ||
       sha(&excel_path)?!=manifest["excelSha256"].as_str().unwrap_or("") {
        return Err("Checksum de los archivos de respaldo incorrecto".into());
    }
    let conn=Connection::open_with_flags(&database,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
    verify_db(&conn)?;
    let sqlite_snapshot=store::read(&conn)?;
    // A manifest must describe the exact committed SQLite snapshot. Digest
    // equality alone does not prove its revision/count metadata is accurate.
    let expected_revision=manifest["revision"].as_u64().ok_or(
        "El manifiesto no tiene una revisión válida")?;
    let expected_clients=manifest["clients"].as_u64().ok_or(
        "El manifiesto no tiene una cantidad válida de clientes")?;
    let expected_vehicles=manifest["vehicles"].as_u64().ok_or(
        "El manifiesto no tiene una cantidad válida de vehículos")?;
    let expected_orders=manifest["orders"].as_u64().ok_or(
        "El manifiesto no tiene una cantidad válida de trabajos")?;
    if expected_revision!=sqlite_snapshot.revision||
       expected_clients!=sqlite_snapshot.data.clients_count() as u64||
       expected_vehicles!=sqlite_snapshot.data.vehicles_count() as u64||
       expected_orders!=sqlite_snapshot.data.orders_count() as u64{
        return Err("El manifiesto no coincide con la revisión o los registros de SQLite".into());
    }
    let workbook=excel::from_xlsx(&fs::read(&excel_path).map_err(err)?)?;
    if excel::canonical_bytes(&sqlite_snapshot.data)?!=excel::canonical_bytes(&workbook)? {
        return Err("Los datos de Excel y SQLite difieren".into());
    }
    Ok(())
}
/// Conservatively reject a "secondary" backup on the same filesystem/volume.
/// A separate volume does NOT itself prove another physical device.
pub(crate) fn check_distinct_volume(primary:&Path,secondary:&Path)->Result<(),String>{
    #[cfg(windows)]
    {
        use std::path::Component;
        let root=|p:&Path|p.components().find_map(|c|match c{
            Component::Prefix(prefix)=>Some(prefix.as_os_str().to_os_string()),
            _=>None,
        });
        let a=root(primary).ok_or("No se pudo determinar la unidad principal")?;
        let b=root(secondary).ok_or("No se pudo determinar la unidad del respaldo")?;
        if a==b{return Err("Elegí otra unidad o disco. Una carpeta en la misma unidad no protege ante fallas.".into());}
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a=fs::metadata(primary).map_err(err)?.dev();
        let b=fs::metadata(secondary).map_err(err)?.dev();
        if a==b{return Err("La copia debe estar en otro volumen o dispositivo, no en la misma unidad.".into());}
    }
    Ok(())
}

fn external_config_path(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let folder=app.path().app_local_data_dir().map_err(err)?;
    fs::create_dir_all(&folder).map_err(err)?;
    Ok(folder.join("external-backup-destination.json"))
}
pub(crate) fn configured_folder(app:&tauri::AppHandle)->Result<Option<PathBuf>,String>{
    let cfg=external_config_path(app)?;
    if !cfg.exists(){return Ok(None);}
    let doc:serde_json::Value=serde_json::from_slice(&fs::read(cfg).map_err(err)?).map_err(err)?;
    let dir=doc["directory"].as_str().ok_or("Destino externo inválido")?;
    if dir.is_empty(){return Ok(None);}
    Ok(Some(PathBuf::from(dir)))
}
#[tauri::command]
pub(crate) fn configure_external_backup(app:tauri::AppHandle)->Result<Option<String>,String>{
    let Some(folder)=rfd::FileDialog::new().set_title("Carpeta o disco de respaldo externo").pick_folder() else{return Ok(None)};
    let destination=fs::canonicalize(&folder).map_err(err)?;
    let local=app.path().app_local_data_dir().map_err(err)?;
    fs::create_dir_all(&local).map_err(err)?;
    let local=fs::canonicalize(local).map_err(err)?;
    if destination.starts_with(&local)||local.starts_with(&destination){
        return Err("La carpeta elegida no puede contener los datos de AutoTrace".into());
    }
    check_distinct_volume(&local,&destination)?;
    if drive_backup::overlaps_encrypted_destination(&app,&destination)?{
        return Err("La carpeta de Google Drive cifrada debe estar aislada: no se puede usar también como destino de respaldos SIN CIFRAR.".into());
    }
    let config=external_config_path(&app)?;
    fs::write(&config,serde_json::to_vec(&serde_json::json!({"directory":destination})).map_err(err)?).map_err(err)?;
    Ok(Some(destination.display().to_string()))
}
fn mirror_pair(result:&BackupResult,directory:&Path)->Result<(),String>{
    if !directory.is_dir(){return Err("La carpeta externa no está disponible".into());}
    let primary=Path::new(&result.manifest_path);
    let stem=primary.file_name().ok_or("Nombre de respaldo inválido")?.to_string_lossy();
    let base=stem.strip_suffix(".manifest.json").ok_or("Nombre de manifiesto inválido")?;
    let files=[
        (Path::new(&result.sqlite_path),directory.join(format!("{base}.sqlite3"))),
        (Path::new(&result.excel_path),directory.join(format!("{base}.xlsx"))),
        (primary,directory.join(format!("{base}.manifest.json")))
    ];
    if files.iter().any(|(_,p)|p.exists()){return Err("El respaldo ya existe en destino".into());}
    let staging=directory.join(format!(".{base}.partial"));
    fs::create_dir(&staging).map_err(err)?;
    let mut owned=Vec::new();
    let result_copy=(||{
        for(i,(from,_))in files.iter().enumerate(){
            let tmp=staging.join(format!("{i}.part"));
            fs::copy(from,&tmp).map_err(err)?;
            if sha(from)?!=sha(&tmp)?{return Err("La copia externa no coincide".into());}
        }
        // Publicar el manifiesto último permite detectar pares incompletos.
        for(i,(_,dst))in files.iter().enumerate(){
            publish_new_file(&staging.join(format!("{i}.part")),dst,&mut owned)?;
        }
        verify_pair_marker(&files[2].1)
    })();
    let _=fs::remove_dir_all(staging);
    if result_copy.is_err(){rollback_owned_files(owned);}
    result_copy
}
fn latest_verified(directory:&Path)->Result<Option<u128>,String>{
    if !directory.is_dir(){return Ok(None);}
    let mut entries=Vec::new();
    for item in fs::read_dir(directory).map_err(err)?{
        let entry=item.map_err(err)?;
        if !entry.file_name().to_string_lossy().ends_with(".manifest.json"){continue;}
        let age=entry.metadata().map_err(err)?.modified().map_err(err)?.duration_since(UNIX_EPOCH).map_err(err)?.as_millis();
        entries.push((age,entry.path()));
    }
    entries.sort_by(|a,b|b.0.cmp(&a.0));
    for (time,path) in entries {
        if verify_pair_marker(&path).is_ok(){return Ok(Some(time));}
    }
    Ok(None)
}

/// Retention uses verified, app-owned snapshot triplets only. Unrecognized files,
/// damaged copies, and manual exports are NEVER removed.
const RECENT_KEEP: usize=12;
const DAY_KEEP: usize=14;
const WEEK_KEEP: usize=8;
const MONTH_KEEP: usize=12;

fn keep_snapshot_times(mut times:Vec<u128>)->HashSet<u128>{
    times.sort_unstable_by(|a,b|b.cmp(a));
    times.dedup();
    let mut keep=HashSet::new();
    let mut day=HashSet::new();
    let mut week=HashSet::new();
    let mut month=HashSet::new();
    for (i,ts) in times.into_iter().enumerate(){
        let Ok(ms)=i64::try_from(ts) else{keep.insert(ts);continue};
        let Some(dt)=DateTime::<Utc>::from_timestamp_millis(ms) else{keep.insert(ts);continue};
        let day_id=(dt.year(),dt.ordinal());
        let week_id=(dt.iso_week().year(),dt.iso_week().week());
        let month_id=(dt.year(),dt.month());
        let fresh_day= !day.contains(&day_id)&&day.len()<DAY_KEEP;
        let fresh_week= !week.contains(&week_id)&&week.len()<WEEK_KEEP;
        let fresh_month= !month.contains(&month_id)&&month.len()<MONTH_KEEP;
        if i<RECENT_KEEP||fresh_day||fresh_week||fresh_month{
            keep.insert(ts);
            if day.len()<DAY_KEEP {day.insert(day_id);}
            if week.len()<WEEK_KEEP {week.insert(week_id);}
            if month.len()<MONTH_KEEP {month.insert(month_id);}
        }
    }
    keep
}
fn snapshot_stamp(name:&str)->Option<u128>{
    let base=name.strip_prefix("AutoTrace_")?.strip_suffix(".manifest.json")?;
    let (number,uuid)=base.split_once('-')?;
    if number.len()<12||number.len()>16||uuid.is_empty()||!uuid.chars().all(|c|c.is_ascii_hexdigit()){
        return None;
    }
    number.parse().ok()
}
/// Old verified triplets beyond the retention policy can be removed. This is
/// conservative: any verification or parsing failure means preserve the files.
fn prune_verified(dir:&Path)->Result<usize,String>{
    if !dir.is_dir(){return Ok(0);}
    let mut valid:Vec<(u128,PathBuf)>=Vec::new();
    for entry in fs::read_dir(dir).map_err(err)?{
        let entry=entry.map_err(err)?;
        let name=entry.file_name();
        let Some(name)=name.to_str() else{continue};
        let Some(stamp)=snapshot_stamp(name) else{continue};
        if verify_pair_marker(&entry.path()).is_ok(){valid.push((stamp,entry.path()));}
    }
    let keep=keep_snapshot_times(valid.iter().map(|(s,_)|*s).collect());
    let mut removed=0;
    for (stamp,manifest) in valid{
        if keep.contains(&stamp){continue;}
        let Some(base)=manifest.file_name().and_then(|s|s.to_str())
            .and_then(|s|s.strip_suffix(".manifest.json")) else{continue};
        let db=dir.join(format!("{base}.sqlite3"));
        let excel=dir.join(format!("{base}.xlsx"));
        // Marker first: interruption cannot leave a formally complete triplet.
        fs::remove_file(manifest).map_err(err)?;
        fs::remove_file(db).map_err(err)?;
        fs::remove_file(excel).map_err(err)?;
        removed+=1;
    }
    Ok(removed)
}

#[tauri::command]
pub(crate) async fn backup_now(app:tauri::AppHandle)->Result<BackupResult,String>{
    tauri::async_runtime::spawn_blocking(move ||backup_now_blocking(app))
        .await.map_err(|_|"Falló la ejecución del respaldo".to_string())?
}
fn backup_now_blocking(app:tauri::AppHandle)->Result<BackupResult,String>{
    let src=store::open_app(&app)?;
    let id=format!("{}-{}",now_ms()?,uuid::Uuid::new_v4().simple());
    let mut backup=backup_pair(&src,&backup_dir(&app)?,&id)?;
    if let Some(folder)=configured_folder(&app)?{
        let main=backup_dir(&app)?;
        let copy=drive_backup::overlaps_encrypted_destination(&app,&folder).and_then(|overlap|{
            if overlap{
                Err("Se bloqueó una copia sin cifrar a la carpeta configurada para Google Drive.".into())
            }else{
                check_distinct_volume(&main,&folder).and_then(|()|mirror_pair(&backup,&folder))
            }
        });
        match copy{
            Ok(())=>{
                backup.external_copy_ok=true;
                if let Err(e)=prune_verified(&folder){eprintln!("AutoTrace external retention: {e}");}
            },
            Err(e)=>backup.external_copy_error=Some(e),
        }
    }
    match drive_backup::replicate_if_unlocked(&app,&backup){
        Ok(Some(_))=>backup.drive_copy_ok=true,
        Ok(None)=>{},
        Err(e)=>backup.drive_copy_error=Some(e),
    }
    match drive_api::replicate_if_connected(&app,&backup){
        Ok(Some(_))=>backup.cloud_copy_ok=true,
        Ok(None)=>{},
        Err(e)=>backup.cloud_copy_error=Some(e),
    }
    if let Err(e)=prune_verified(&backup_dir(&app)?){eprintln!("AutoTrace local retention: {e}");}
    Ok(backup)
}
#[tauri::command]
pub(crate) fn backup_status(app:tauri::AppHandle)->Result<BackupStatus,String>{
    let local=backup_dir(&app)?;
    let external=configured_folder(&app)?;
    let last_external=if let Some(folder)=external.as_ref(){
        if check_distinct_volume(&local,folder).is_ok(){latest_verified(folder)?}else{None}
    }else{None};
    Ok(BackupStatus{
        last_backup_unix_ms:latest_verified(&local)?,
        local_only:last_external.is_none(),
        directory:local.display().to_string(),
        external_directory:external.map(|p|p.display().to_string()),
        last_external_backup_unix_ms:last_external
    })
}
/// Publish a fully generated Excel export on the selected filesystem only
/// after its bytes are complete and durable. Use the same atomic no-clobber
/// operation as encrypted backup publication (MoveFileExW on Windows / hard
/// link on POSIX), never a check-then-rename that could overwrite a competitor.
fn publish_excel_export_with<F>(
    destination:&Path,bytes:&[u8],before_publish:F
)->Result<(),String> where F:FnOnce()->Result<(),String>{
    let parent=destination.parent().ok_or("Destino de Excel inválido")?;
    let stage=parent.join(format!(".autotrace-excel-{}.partial",uuid::Uuid::new_v4()));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup{
        fn drop(&mut self){let _=fs::remove_file(&self.0);}
    }
    let _cleanup=Cleanup(stage.clone());
    let mut out=fs::OpenOptions::new().write(true).create_new(true)
        .open(&stage).map_err(err)?;
    use std::io::Write;
    out.write_all(bytes).map_err(err)?;
    out.sync_all().map_err(err)?;
    drop(out);
    if sha(&stage)?!=format!("{:x}",Sha256::digest(bytes)){
        return Err("Verificación del Excel exportado fallida".into());
    }
    before_publish()?;
    crate::drive_transport::publish_verified_cipher(&stage,destination)
}
fn publish_excel_export(destination:&Path,bytes:&[u8])->Result<(),String>{
    publish_excel_export_with(destination,bytes,||Ok(()))
}
#[tauri::command]
pub(crate) fn export_excel_dialog(app:tauri::AppHandle)->Result<bool,String>{
    let file=rfd::FileDialog::new().set_file_name("AutoTrace_completo.xlsx").add_filter("Excel AutoTrace", &["xlsx"]).save_file();
    let Some(path)=file else{return Ok(false)};
    if path.exists(){return Err("Para proteger archivos existentes, elegí otro nombre de Excel".into());}
    let dir=path.parent().ok_or("Destino de Excel inválido")?;
    let src=store::open_app(&app)?;
    let id=format!("{}-{}",now_ms()?,uuid::Uuid::new_v4().simple());
    let staging=dir.join(format!(".autotrace-export-{id}.sqlite3"));
    let result=(||{
        snapshot(&src,&staging)?;
        let saved=store::read(&Connection::open(&staging).map_err(err)?)?;
        let bytes=excel::to_xlsx(&saved.data)?;
        excel::from_xlsx(&bytes)?;
        publish_excel_export(&path,&bytes)?;
        Ok(true)
    })();
    let _=fs::remove_file(staging);
    result
}
#[tauri::command]
pub(crate) fn export_customer_excel_dialog(app:tauri::AppHandle,client_id:String)->Result<bool,String>{
    let source=store::read(&store::open_app(&app)?)?;
    let bytes=crate::client_report::to_customer_xlsx(&source.data,&client_id)?;
    let Some(path)=rfd::FileDialog::new().set_file_name("AutoTrace_informe_cliente.xlsx")
        .add_filter("Excel para cliente",&["xlsx"]).save_file() else{return Ok(false)};
    if path.exists(){return Err("Elegí un archivo nuevo: el informe no sobrescribe archivos existentes".into());}
    publish_excel_export(&path,&bytes)?;
    Ok(true)
}
// Only data from a verified and validated SQLite source enters the current app.
// Copying a user-selected SQLite database directly to the live path is forbidden.
pub(crate) fn read_sqlite_candidate(path:&Path)->Result<(store::AppData,bool),String>{
    let meta=fs::metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len()>512_000_000 || meta.len()<100 {
        return Err("Archivo SQLite vacío o demasiado grande".into());
    }
    let mut verified=false;
    let marker=path.with_extension("manifest.json");
    if marker.exists(){
        bounded_backup_file(&marker,MAX_VERIFIED_MANIFEST_BYTES)?;
        let doc:serde_json::Value=serde_json::from_slice(&fs::read(&marker).map_err(err)?).map_err(err)?;
        if doc["kind"]!="autotrace-dual-backup" || doc["version"]!=1 ||
            doc["sqliteSha256"].as_str()!=Some(sha(path)?.as_str()) {
            return Err("Manifest o SHA-256 del backup no coinciden".into());
        }
        let stem=path.file_stem().and_then(|s|s.to_str()).ok_or("Nombre inválido")?;
        let id=doc["snapshotId"].as_str().ok_or("Falta ID del backup")?;
        if stem!=format!("AutoTrace_{id}") {return Err("ID de respaldo incorrecto".into());}
        // A matching manifest represents the entire SQLite+Excel pair. A
        // partial/corrupted XLSX makes it invalid as a verified backup.
        verify_pair_marker(&marker)?;
        verified=true;
    }
    let conn=Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
    let version:i64=conn.query_row("PRAGMA user_version",[],|r|r.get(0)).map_err(err)?;
    if version!=1 {return Err("Versión de base SQLite no admitida".into());}
    verify_db(&conn)?;
    Ok((store::read(&conn)?.data,verified))
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct SqlitePreview {
    pub data:store::AppData,
    pub filename:String,
    pub verified:bool,
}
#[tauri::command]
pub(crate) fn choose_sqlite_import()->Result<Option<SqlitePreview>,String>{
    let Some(path)=rfd::FileDialog::new().add_filter("Respaldo SQLite AutoTrace",&["sqlite3"]).pick_file() else{
        return Ok(None);
    };
    let (data,verified)=read_sqlite_candidate(&path)?;
    Ok(Some(SqlitePreview{
        data,verified,
        filename:path.file_name().unwrap_or_default().to_string_lossy().into_owned()
    }))
}
#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct RecoveryTestResult{
    pub filename:String,
    pub clients:usize,
    pub vehicles:usize,
    pub orders:usize,
    pub sqlite_and_excel_match:bool,
}
/// Destructive-free restore rehearsal: independently import SQLite and Excel
/// into two FRESH SQLite databases and compare canonical data with snapshot.
pub(crate) fn recovery_rehearsal(manifest_path:&Path)->Result<RecoveryTestResult,String>{
    verify_pair_marker(manifest_path)?;
    let doc:serde_json::Value=serde_json::from_slice(&fs::read(manifest_path).map_err(err)?).map_err(err)?;
    let id=doc["snapshotId"].as_str().ok_or("Respaldo sin identificador")?;
    let dir=manifest_path.parent().ok_or("Respaldo sin carpeta")?;
    let sql_path=dir.join(format!("AutoTrace_{id}.sqlite3"));
    let xlsx_path=dir.join(format!("AutoTrace_{id}.xlsx"));
    // This also validates schema version and app-level constraints.
    let (source, verified)=read_sqlite_candidate(&sql_path)?;
    if !verified{return Err("Respaldo sin manifiesto verificado".into());}
    let book=excel::from_xlsx(&fs::read(&xlsx_path).map_err(err)?)?;
    for candidate in [&source,&book]{
        let mut fresh=Connection::open_in_memory().map_err(err)?;
        fresh.execute_batch(include_str!("../migrations/001_init.sql")).map_err(err)?;
        store::write(&mut fresh,candidate,0)?;
        verify_db(&fresh)?;
        let loaded=store::read(&fresh)?.data;
        if excel::canonical_bytes(&loaded)?!=excel::canonical_bytes(&source)?{
            return Err("La recuperación en una base nueva no reproduce los datos originales".into());
        }
    }
    Ok(RecoveryTestResult{
        filename:manifest_path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        clients:source.clients_count(),
        vehicles:source.vehicles_count(),
        orders:source.orders_count(),
        sqlite_and_excel_match:true
    })
}
#[tauri::command]
pub(crate) fn test_backup_recovery()->Result<Option<RecoveryTestResult>,String>{
    let Some(manifest)=rfd::FileDialog::new()
        .set_title("Seleccioná el archivo .manifest.json del respaldo")
        .add_filter("Manifiesto AutoTrace",&["json"]).pick_file() else{return Ok(None)};
    recovery_rehearsal(&manifest).map(Some)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct ExcelPreview{pub data:store::AppData,pub filename:String}
#[tauri::command]
pub(crate) fn choose_excel_import()->Result<Option<ExcelPreview>,String>{
    let Some(path)=rfd::FileDialog::new().add_filter("Excel AutoTrace", &["xlsx"]).pick_file() else{return Ok(None)};
    let meta=fs::metadata(&path).map_err(err)?;
    if meta.len()>25_000_000 {return Err("Excel supera 25 MB".into());}
    let bytes=fs::read(&path).map_err(err)?;
    let data=excel::from_xlsx(&bytes)?;
    Ok(Some(ExcelPreview{data,filename:path.file_name().unwrap_or_default().to_string_lossy().into_owned()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retention_keeps_frequent_recent_and_monthly_recovery_points(){
        let mut times=Vec::new();
        // 17 months, 6 consecutive snapshots each month plus a 4-hour cadence
        // on the last day: force daily, weekly and monthly retention to apply.
        for month in 1..=17 {
            let stamp=chrono::NaiveDate::from_ymd_opt(2025+((month-1)/12) as i32, (month-1)%12+1, 15)
                .unwrap().and_hms_opt(12,0,0).unwrap().and_utc().timestamp_millis() as u128;
            for increment in 0..6{times.push(stamp+increment*14_400_000);}
        }
        let keep=keep_snapshot_times(times.clone());
        assert!(keep.len()>=RECENT_KEEP);
        assert!(keep.len()<times.len());
        assert!(keep.contains(times.last().unwrap()));
        assert!(!keep.contains(&times[0]));
    }
    #[test]
    fn only_verified_triplets_may_be_pruned(){
        let tmp=std::env::temp_dir().join(format!("autotrace-retention-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let base=chrono::NaiveDate::from_ymd_opt(2024,1,3).unwrap()
            .and_hms_opt(12,0,0).unwrap().and_utc().timestamp_millis() as u128;
        for i in 0..17 {
            let id=format!("{}-{}",base+(i as u128)*1000,uuid::Uuid::new_v4().simple());
            backup_pair(&db,&tmp,&id).unwrap();
        }
        // A malformed and a user-owned file must survive cleanup.
        let private=tmp.join("informe_cliente.xlsx");
        fs::write(&private,b"keep").unwrap();
        let corrupt=tmp.join(format!("AutoTrace_{}-deadbeef.manifest.json",base-86_400_000));
        fs::write(&corrupt,b"bad").unwrap();
        let deleted=prune_verified(&tmp).unwrap();
        assert!(deleted>0);
        assert!(private.exists());
        assert!(corrupt.exists());
        assert!(latest_verified(&tmp).unwrap().is_some());
        fs::remove_dir_all(tmp).unwrap();
    }
    #[test]
    fn clean_room_restore_from_both_sqlite_and_excel_preserves_realistic_records(){
        let location=std::env::temp_dir().join(format!("autotrace-clean-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&location).unwrap();
        let payload=serde_json::json!({
            "clients":[{"id":"c-A","name":"María de Prueba","phone":"01100123","email":"","notes":"Interno","createdAt":"2026-10-20"}],
            "vehicles":[{"id":"v-A","clientId":"c-A","plate":"AF012BC","make":"Ford","model":"Fiesta","year":2017,"mileage":75123.0,"vin":"","color":"Azul","nextServiceDate":"2026-12-21","notes":"","createdAt":"2026-10-20"}],
            "orders":[{"id":"o-A","vehicleId":"v-A","title":"Diagnóstico","category":"Electrónica","status":"delivered","openedAt":"2026-10-21","completedAt":"2026-10-22","mileage":75123.0,"description":"Revisar señal","partsCost":750.29,"laborCost":540.11,"charged":3500.5,"paid":2010.1,"notes":"Nota taller","createdAt":"2026-10-21"}],
            "settings":{"garageName":"Mi taller de prueba","currency":"ARS","demo":false}
        });
        let original:store::AppData=serde_json::from_value(payload.clone()).unwrap();
        store::validate(&original).unwrap();
        let mut source=Connection::open_in_memory().unwrap();
        source.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        store::write(&mut source,&original,0).unwrap();
        let done=backup_pair(&source,&location,"fixture").unwrap();
        drop(source); // Simula perder por completo la computadora original.

        let (from_sqlite,verified)=read_sqlite_candidate(Path::new(&done.sqlite_path)).unwrap();
        assert!(verified);
        let from_excel=excel::from_xlsx(&fs::read(&done.excel_path).unwrap()).unwrap();
        for candidate in [from_sqlite,from_excel]{
            let mut new_pc=Connection::open_in_memory().unwrap();
            new_pc.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
            store::write(&mut new_pc,&candidate,0).unwrap();
            let restored=store::read(&new_pc).unwrap().data;
            assert_eq!(serde_json::to_value(restored).unwrap(),payload);
            assert_eq!(new_pc.query_row("SELECT count(*) FROM pragma_foreign_key_check",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        }
        // If the XLSX is modified the SQLite candidate cannot claim the
        // SHA-verified paired backup is fully valid.
        fs::write(&done.excel_path,b"damaged").unwrap();
        assert!(read_sqlite_candidate(Path::new(&done.sqlite_path)).is_err());
        fs::remove_dir_all(location).unwrap();
    }
    #[test]
    fn recovery_rehearsal_uses_two_fresh_databases_and_rejects_damage(){
        let temp=std::env::temp_dir().join(format!("autotrace-rehearsal-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let source=Connection::open_in_memory().unwrap();
        source.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let pair=backup_pair(&source,&temp,"fixture").unwrap();
        let report=recovery_rehearsal(Path::new(&pair.manifest_path)).unwrap();
        assert!(report.sqlite_and_excel_match);
        assert_eq!(report.clients,0);
        fs::write(&pair.excel_path,b"tampered").unwrap();
        assert!(recovery_rehearsal(Path::new(&pair.manifest_path)).is_err());
        fs::remove_dir_all(&temp).unwrap();
    }
    #[test]
    fn same_volume_cannot_be_selected_as_independent_backup(){
        let base=std::env::temp_dir();
        assert!(check_distinct_volume(&base,&base.join("secondary")).is_err());
    }
    #[test]
    fn manifest_revision_and_record_counts_must_match_actual_sqlite_snapshot(){
        let tmp=std::env::temp_dir().join(format!("autotrace-manifest-audit-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let mut original=Connection::open_in_memory().unwrap();
        original.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let payload:store::AppData=serde_json::from_value(serde_json::json!({
          "clients":[{"id":"c1","name":"Cliente sintético","phone":"","email":"","notes":"","createdAt":""}],
          "vehicles":[{"id":"v1","clientId":"c1","plate":"AF123BC","make":"Prueba","model":"Prueba","year":2020,
                      "mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[],
          "settings":{"garageName":"Taller QA","currency":"ARS","demo":false}
        })).unwrap();
        store::write(&mut original,&payload,0).unwrap();
        let pair=backup_pair(&original,&tmp,"fixture").unwrap();
        let manifest=Path::new(&pair.manifest_path);
        let original_bytes=fs::read(manifest).unwrap();
        assert!(verify_pair_marker(manifest).is_ok());
        for (field,value) in [
            ("revision",serde_json::json!(0)),
            ("clients",serde_json::json!(2)),
            ("vehicles",serde_json::json!(0)),
            ("orders",serde_json::json!(1)),
            ("revision",serde_json::json!("invalid")),
        ]{
            let mut metadata:serde_json::Value=serde_json::from_slice(&original_bytes).unwrap();
            metadata[field]=value;
            fs::write(manifest,serde_json::to_vec(&metadata).unwrap()).unwrap();
            assert!(verify_pair_marker(manifest).is_err(),
                "inconsistent manifest metadata must fail closed: {field}");
            assert!(recovery_rehearsal(manifest).is_err(),
                "inconsistent backup must not pass clean-room restore rehearsal");
        }
        fs::write(manifest,&original_bytes).unwrap();
        assert!(verify_pair_marker(manifest).is_ok());
        assert!(recovery_rehearsal(manifest).unwrap().sqlite_and_excel_match);
        fs::remove_dir_all(tmp).unwrap();
    }
    #[test]
    fn complete_and_customer_excel_exports_never_overwrite_concurrent_destinations(){
        let root=std::env::temp_dir().join(format!("autotrace-excel-race-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let target=root.join("AutoTrace_completo.xlsx");
        let report=root.join("AutoTrace_informe_cliente.xlsx");
        let bytes=b"synthetic Excel fixture; publication semantics only";
        // Inject the competing writer AFTER the staged export is synced and
        // verified, in the exact check-then-rename race window.
        let result=publish_excel_export_with(&target,bytes,||{
            fs::write(&target,b"existing important user workbook").map_err(err)
        });
        assert!(result.is_err(),"an existing Excel may not be replaced by an export");
        assert_eq!(fs::read(&target).unwrap(),b"existing important user workbook");
        assert_eq!(fs::read_dir(&root).unwrap().count(),1,
            "a rejected Excel export must not leak staging files");
        publish_excel_export(&report,bytes).unwrap();
        assert_eq!(fs::read(&report).unwrap(),bytes);
        assert!(publish_excel_export(&report,b"another export").is_err(),
            "repeating an export with the same name cannot overwrite its original");
        assert_eq!(fs::read(&report).unwrap(),bytes);
        assert_eq!(fs::read_dir(&root).unwrap().count(),2,
            "successful export must leave only final Excel files, no .partial");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn simulated_full_disk_after_partial_write_rolls_back_without_touching_prior_backup(){
        use std::io::{Read,Write};
        let root=std::env::temp_dir().join(format!("autotrace-no-space-{}",uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source=root.join("source.synthetic");
        let partial=root.join("AutoTrace_new.sqlite3");
        let prior=root.join("AutoTrace_older.sqlite3");
        let marker=root.join("AutoTrace_new.manifest.json");
        fs::write(&source,b"synthetic-workshop-bytes-without-private-data").unwrap();
        fs::write(&prior,b"previous-good-backup").unwrap();
        let mut owned=Vec::new();
        // Inject a disk-full-like error after the destination was exclusively
        // created and some bytes have reached it. This exercises the exact
        // same ownership/rollback mechanism as a real partial fs::copy error.
        let result=publish_new_file_with(&source,&partial,&mut owned,|input,out|{
            let mut prefix=[0u8;8];
            input.read_exact(&mut prefix)?;
            out.write_all(&prefix)?;
            Err(std::io::Error::other("synthetic ENOSPC after partial write"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&partial).unwrap(),b"syntheti");
        assert_eq!(owned,vec![partial.clone()]);
        assert!(!marker.exists(),"completion marker must not be published on I/O failure");
        rollback_owned_files(owned);
        assert!(!partial.exists(),"partial SQLite must be removed after failed publication");
        assert_eq!(fs::read(&prior).unwrap(),b"previous-good-backup",
            "failure rollback must never destroy an existing valid backup");
        // A later healthy backup may retry with the same previously failed name.
        let mut retry_owned=Vec::new();
        publish_new_file(&source,&partial,&mut retry_owned).unwrap();
        assert_eq!(fs::read(&partial).unwrap(),fs::read(&source).unwrap());
        assert_eq!(fs::read(&prior).unwrap(),b"previous-good-backup");
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn file_promotion_refuses_existing_destination_and_tracks_only_owned_files(){
        let temp=std::env::temp_dir().join(format!("autotrace-publish-race-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let source=temp.join("synthetic-source.part");
        let old=temp.join("protected-prior.sqlite3");
        let new=temp.join("new-copy.sqlite3");
        fs::write(&source,b"new-synthetic-payload").unwrap();
        fs::write(&old,b"trusted-preexisting-backup").unwrap();
        let mut owned=Vec::new();
        assert!(publish_new_file(&source,&old,&mut owned).is_err(),
            "create_new must refuse existing backup even in a cross-process race");
        assert!(owned.is_empty(),"an existing file must never be marked as our cleanup target");
        assert_eq!(fs::read(&old).unwrap(),b"trusted-preexisting-backup");
        publish_new_file(&source,&new,&mut owned).unwrap();
        assert_eq!(owned,vec![new.clone()]);
        assert_eq!(fs::read(&new).unwrap(),b"new-synthetic-payload");
        assert!(publish_new_file(&source,&new,&mut owned).is_err());
        assert_eq!(owned,vec![new.clone()],"failed reservation must not gain ownership");
        for path in owned{fs::remove_file(path).unwrap();}
        assert!(old.exists(),"cleanup must preserve prior backup");
        assert_eq!(fs::read(&old).unwrap(),b"trusted-preexisting-backup");
        assert!(!new.exists());
        fs::remove_dir_all(&temp).unwrap();
    }
    #[test]
    fn oversized_external_manifest_and_excel_are_rejected_before_loading(){
        let root=std::env::temp_dir().join(format!("autotrace-bounded-backups-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source=Connection::open_in_memory().unwrap();
        source.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let pair=backup_pair(&source,&root,"fixture").unwrap();
        let manifest=Path::new(&pair.manifest_path);
        let sqlite=Path::new(&pair.sqlite_path);
        let workbook=Path::new(&pair.excel_path);
        assert!(verify_pair_marker(manifest).is_ok());
        let original_manifest=fs::read(manifest).unwrap();
        fs::OpenOptions::new().write(true).open(manifest).unwrap()
            .set_len(MAX_VERIFIED_MANIFEST_BYTES+1).unwrap();
        assert!(verify_pair_marker(manifest).unwrap_err().contains("tamaño fuera de límites"));
        assert!(read_sqlite_candidate(sqlite).err().expect("oversized marker must fail")
            .contains("tamaño fuera de límites"));
        fs::write(manifest,&original_manifest).unwrap();
        let original_excel=fs::read(workbook).unwrap();
        fs::OpenOptions::new().write(true).open(workbook).unwrap()
            .set_len(MAX_VERIFIED_EXCEL_BYTES+1).unwrap();
        assert!(verify_pair_marker(manifest).unwrap_err().contains("tamaño fuera de límites"));
        fs::write(workbook,&original_excel).unwrap();
        assert!(verify_pair_marker(manifest).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn backup_pair_contains_restorable_sqlite_and_excel() {
        let temp=std::env::temp_dir().join(format!("autotrace-test-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let source=Connection::open_in_memory().unwrap();
        source.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let done=backup_pair(&source,&temp,"fixture").unwrap();
        let manifest:serde_json::Value=serde_json::from_slice(&fs::read(&done.manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["kind"],"autotrace-dual-backup");
        assert_eq!(manifest["sqliteSha256"],sha(Path::new(&done.sqlite_path)).unwrap());
        assert_eq!(manifest["excelSha256"],sha(Path::new(&done.excel_path)).unwrap());
        let restored=excel::from_xlsx(&fs::read(&done.excel_path).unwrap()).unwrap();
        assert_eq!(restored.clients_count(),0);
        assert!(backup_pair(&source,&temp,"fixture").is_err());
        assert!(verify_pair_marker(Path::new(&done.manifest_path)).is_ok());
        let counterfeit=temp.join("AutoTrace_1234567890123-0a.manifest.json");
        fs::copy(&done.manifest_path,&counterfeit).unwrap();
        assert!(verify_pair_marker(&counterfeit).is_err(),"no aceptar manifiesto con identidad ajena");
        let usb=temp.join("usb");fs::create_dir(&usb).unwrap();
        mirror_pair(&done,&usb).unwrap();
        assert!(latest_verified(&usb).unwrap().is_some());
        assert!(read_sqlite_candidate(Path::new(&done.sqlite_path)).unwrap().1);
        fs::write(&done.excel_path,b"malformado").unwrap();
        assert!(latest_verified(&temp).unwrap().is_none());
        assert!(latest_verified(&usb).unwrap().is_some());
        // Un SQLite cuyo Excel emparejado se dañó NO es un respaldo completo verificado.
        assert!(read_sqlite_candidate(Path::new(&done.sqlite_path)).is_err());
        let broken=temp.join("broken.sqlite3");fs::write(&broken,b"corrupt").unwrap();
        assert!(read_sqlite_candidate(&broken).is_err());
        fs::remove_dir_all(temp).unwrap();
    }
}
