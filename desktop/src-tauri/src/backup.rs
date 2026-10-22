//! Two matching, locally verified artifacts. Does not claim off-device durability.
use crate::{excel, store};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::{Path,PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};
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
}
fn err(e:impl std::fmt::Display)->String {format!("No se pudo crear o verificar el respaldo: {e}")}
fn now_ms()->Result<u128,String> {Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_millis())}
fn backup_dir(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let dir=app.path().app_local_data_dir().map_err(err)?.join("backups");
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir)
}
fn sha(path:&Path)->Result<String,String>{let bytes=fs::read(path).map_err(err)?;Ok(format!("{:x}",Sha256::digest(bytes)))}
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
fn backup_pair(src:&Connection, dest:&Path, id:&str)->Result<BackupResult,String>{
    fs::create_dir_all(dest).map_err(err)?;
    let db=dest.join(format!("AutoTrace_{id}.sqlite3"));
    let excel=dest.join(format!("AutoTrace_{id}.xlsx"));
    let manifest=dest.join(format!("AutoTrace_{id}.manifest.json"));
    if [db.as_path(),excel.as_path(),manifest.as_path()].iter().any(|p|p.exists()) {return Err("Ya existe un respaldo con ese identificador".into());}
    let staging=dest.join(format!(".autotrace-{id}.partial"));
    fs::create_dir(&staging).map_err(err)?;
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
        // Manifest acts as completion marker and is always promoted last.
        fs::rename(&temp_db,&db).map_err(err)?;
        fs::rename(&temp_xlsx,&excel).map_err(err)?;
        fs::rename(&temp_manifest,&manifest).map_err(err)?;
        if sha(&db)?!=metadata["sqliteSha256"].as_str().unwrap_or("") || sha(&excel)?!=metadata["excelSha256"].as_str().unwrap_or("") {return Err("Checksum no coincide después de copiar".into());}
        Ok(BackupResult{sqlite_path:db.display().to_string(),excel_path:excel.display().to_string(),manifest_path:manifest.display().to_string(),external_copy_ok:false,external_copy_error:None})
    })();
    let _=fs::remove_dir_all(staging);
    if result.is_err(){let _=fs::remove_file(db);let _=fs::remove_file(excel);let _=fs::remove_file(manifest);}
    result
}
fn verify_pair_marker(path:&Path)->Result<(),String>{
    let manifest:serde_json::Value=serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
    if manifest["kind"]!="autotrace-dual-backup" || manifest["version"]!=1 {
        return Err("Manifiesto de respaldo no reconocido".into());
    }
    let id=manifest["snapshotId"].as_str().ok_or("Manifiesto sin ID")?;
    if id.is_empty()||id.len()>120||!id.chars().all(|c|c.is_ascii_alphanumeric()||c=='-') {
        return Err("ID de respaldo inválido".into());
    }
    let dir=path.parent().ok_or("Carpeta de respaldo inválida")?;
    let database=dir.join(format!("AutoTrace_{id}.sqlite3"));
    let excel_path=dir.join(format!("AutoTrace_{id}.xlsx"));
    if sha(&database)?!=manifest["sqliteSha256"].as_str().unwrap_or("") ||
       sha(&excel_path)?!=manifest["excelSha256"].as_str().unwrap_or("") {
        return Err("Checksum de los archivos de respaldo incorrecto".into());
    }
    let conn=Connection::open_with_flags(&database,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
    verify_db(&conn)?;
    let sqlite_data=store::read(&conn)?.data;
    let workbook=excel::from_xlsx(&fs::read(&excel_path).map_err(err)?)?;
    if excel::canonical_bytes(&sqlite_data)?!=excel::canonical_bytes(&workbook)? {
        return Err("Los datos de Excel y SQLite difieren".into());
    }
    Ok(())
}
fn external_config_path(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let folder=app.path().app_local_data_dir().map_err(err)?;
    fs::create_dir_all(&folder).map_err(err)?;
    Ok(folder.join("external-backup-destination.json"))
}
fn configured_folder(app:&tauri::AppHandle)->Result<Option<PathBuf>,String>{
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
    let result_copy=(||{
        for(i,(from,_))in files.iter().enumerate(){
            let tmp=staging.join(format!("{i}.part"));
            fs::copy(from,&tmp).map_err(err)?;
            if sha(from)?!=sha(&tmp)?{return Err("La copia externa no coincide".into());}
        }
        // Publicar el manifiesto último permite detectar pares incompletos.
        for(i,(_,dst))in files.iter().enumerate(){
            fs::rename(staging.join(format!("{i}.part")),dst).map_err(err)?;
        }
        verify_pair_marker(&files[2].1)
    })();
    let _=fs::remove_dir_all(staging);
    if result_copy.is_err(){for(_,p)in files.iter(){let _=fs::remove_file(p);}}
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
#[tauri::command]
pub(crate) fn backup_now(app:tauri::AppHandle)->Result<BackupResult,String>{
    let src=store::open_app(&app)?;
    let id=format!("{}-{}",now_ms()?,uuid::Uuid::new_v4().simple());
    let mut backup=backup_pair(&src,&backup_dir(&app)?,&id)?;
    if let Some(folder)=configured_folder(&app)?{
        match mirror_pair(&backup,&folder){
            Ok(())=>backup.external_copy_ok=true,
            Err(e)=>backup.external_copy_error=Some(e),
        }
    }
    Ok(backup)
}
#[tauri::command]
pub(crate) fn backup_status(app:tauri::AppHandle)->Result<BackupStatus,String>{
    let local=backup_dir(&app)?;
    let external=configured_folder(&app)?;
    let last_external=if let Some(folder)=external.as_ref(){latest_verified(folder)?}else{None};
    Ok(BackupStatus{
        last_backup_unix_ms:latest_verified(&local)?,
        local_only:last_external.is_none(),
        directory:local.display().to_string(),
        external_directory:external.map(|p|p.display().to_string()),
        last_external_backup_unix_ms:last_external
    })
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
        let temp=dir.join(format!(".autotrace-export-{id}.tmp"));
        fs::write(&temp,bytes).map_err(err)?;
        if let Err(e)=fs::rename(&temp,&path){let _=fs::remove_file(temp);return Err(err(e));}
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
    let folder=path.parent().ok_or("Destino inválido")?;
    let tmp=folder.join(format!(".autotrace-report-{}.tmp",uuid::Uuid::new_v4()));
    let result=(||{
        fs::write(&tmp,&bytes).map_err(err)?;
        if sha(&tmp)?!=format!("{:x}",Sha256::digest(&bytes)){return Err("Verificación del informe fallida".into());}
        fs::rename(&tmp,&path).map_err(err)?;
        Ok(true)
    })();
    if result.is_err(){let _=fs::remove_file(tmp);}
    result
}
// Only data from a verified and validated SQLite source enters the current app.
// Copying a user-selected SQLite database directly to the live path is forbidden.
fn read_sqlite_candidate(path:&Path)->Result<(store::AppData,bool),String>{
    let meta=fs::metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len()>512_000_000 || meta.len()<100 {
        return Err("Archivo SQLite vacío o demasiado grande".into());
    }
    let mut verified=false;
    let marker=path.with_extension("manifest.json");
    if marker.exists(){
        let doc:serde_json::Value=serde_json::from_slice(&fs::read(&marker).map_err(err)?).map_err(err)?;
        if doc["kind"]!="autotrace-dual-backup" || doc["version"]!=1 ||
            doc["sqliteSha256"].as_str()!=Some(sha(path)?.as_str()) {
            return Err("Manifest o SHA-256 del backup no coinciden".into());
        }
        let stem=path.file_stem().and_then(|s|s.to_str()).ok_or("Nombre inválido")?;
        let id=doc["snapshotId"].as_str().ok_or("Falta ID del backup")?;
        if stem!=format!("AutoTrace_{id}") {return Err("ID de respaldo incorrecto".into());}
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
        let usb=temp.join("usb");fs::create_dir(&usb).unwrap();
        mirror_pair(&done,&usb).unwrap();
        assert!(latest_verified(&usb).unwrap().is_some());
        fs::write(&done.excel_path,b"malformado").unwrap();
        assert!(latest_verified(&temp).unwrap().is_none());
        assert!(latest_verified(&usb).unwrap().is_some());
        assert!(read_sqlite_candidate(Path::new(&done.sqlite_path)).unwrap().1);
        let broken=temp.join("broken.sqlite3");fs::write(&broken,b"corrupt").unwrap();
        assert!(read_sqlite_candidate(&broken).is_err());
        fs::remove_dir_all(temp).unwrap();
    }
}
