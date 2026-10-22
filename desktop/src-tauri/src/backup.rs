//! Two matching, locally verified artifacts. Does not claim off-device durability.
use crate::{excel, store};
use rusqlite::{backup::Backup, Connection};
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
}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct BackupResult {
    pub sqlite_path: String,
    pub excel_path: String,
    pub manifest_path: String,
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
        Ok(BackupResult{sqlite_path:db.display().to_string(),excel_path:excel.display().to_string(),manifest_path:manifest.display().to_string()})
    })();
    let _=fs::remove_dir_all(staging);
    if result.is_err(){let _=fs::remove_file(db);let _=fs::remove_file(excel);let _=fs::remove_file(manifest);}
    result
}
#[tauri::command]
pub(crate) fn backup_now(app:tauri::AppHandle)->Result<BackupResult,String>{
    let src=store::open_app(&app)?;
    let id=format!("{}-{}",now_ms()?,uuid::Uuid::new_v4().simple());
    backup_pair(&src,&backup_dir(&app)?,&id)
}
#[tauri::command]
pub(crate) fn backup_status(app:tauri::AppHandle)->Result<BackupStatus,String>{
    let dir=backup_dir(&app)?;
    let mut last=None;
    for file in fs::read_dir(&dir).map_err(err)?{
        let entry=file.map_err(err)?;
        if entry.path().extension().and_then(|s|s.to_str())!=Some("json") || !entry.file_name().to_string_lossy().ends_with(".manifest.json"){continue;}
        let meta=entry.metadata().map_err(err)?;
        let m=meta.modified().map_err(err)?.duration_since(UNIX_EPOCH).map_err(err)?.as_millis();
        if last.is_none_or(|old|m>old){last=Some(m);}
    }
    Ok(BackupStatus{last_backup_unix_ms:last,local_only:true,directory:dir.display().to_string()})
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
        let restored=excel::from_xlsx(&fs::read(done.excel_path).unwrap()).unwrap();
        assert_eq!(restored.clients_count(),0);
        assert!(backup_pair(&source,&temp,"fixture").is_err());
        fs::remove_dir_all(temp).unwrap();
    }
}
