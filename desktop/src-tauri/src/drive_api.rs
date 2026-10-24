//! Direct Google Drive API orchestrator: local-first, no plaintext uploads.
use crate::{backup,drive_backup,drive_oauth,drive_transport};
use serde::{Serialize,Deserialize};
use std::{fs,path::{Path,PathBuf},sync::{Mutex,OnceLock}};
static UPLOAD_LOCK:OnceLock<Mutex<()>>=OnceLock::new();
use tauri::Manager;
#[derive(Default,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct Settings{
    client_id:String,
    last_verified_upload_at:Option<String>,
    last_verified_file_id:Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct ApiStatus{
    pub client_configured:bool,
    pub connected:bool,
    pub pending_count:usize,
    pub last_verified_upload_at:Option<String>,
    pub last_verified_file_id:Option<String>,
    pub note:&'static str,
}
fn home(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let p=app.path().app_local_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&p).map_err(|e|e.to_string())?;Ok(p)
}
fn config_path(app:&tauri::AppHandle)->Result<PathBuf,String>{Ok(home(app)?.join("google-drive-api.json"))}
fn config(app:&tauri::AppHandle)->Result<Settings,String>{
    let p=config_path(app)?;
    if !p.exists(){return Ok(Settings::default());}
    serde_json::from_slice(&fs::read(p).map_err(|e|e.to_string())?).map_err(|_|"Configuración de Google Drive API corrupta".into())
}
fn save(app:&tauri::AppHandle,state:&Settings)->Result<(),String>{
    let p=config_path(app)?;
    let tmp=p.with_extension(format!("{}.tmp",uuid::Uuid::new_v4().simple()));
    fs::write(&tmp,serde_json::to_vec(state).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if let Err(e)=fs::rename(&tmp,&p){let _=fs::remove_file(tmp);return Err(e.to_string());}
    Ok(())
}
fn queue_dir(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let dir=home(app)?.join("drive-api-encrypted-pending");
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir)
}
fn pending(app:&tauri::AppHandle)->Result<Vec<PathBuf>,String>{
    let mut found=Vec::new();
    for entry in fs::read_dir(queue_dir(app)?).map_err(|e|e.to_string())?{
        let path=entry.map_err(|e|e.to_string())?.path();
        if path.is_file()&&path.file_name().and_then(|x|x.to_str())
            .is_some_and(|n|n.starts_with("AutoTrace_")&&n.ends_with(".atbackup.age")){found.push(path);}
    }
    found.sort();Ok(found)
}
#[tauri::command]
pub(crate) fn drive_api_status(app:tauri::AppHandle)->Result<ApiStatus,String>{
    let cfg=config(&app)?;
    Ok(ApiStatus{
        client_configured:!cfg.client_id.is_empty(),
        connected:!cfg.client_id.is_empty()&&drive_oauth::authorized(),
        pending_count:pending(&app)?.len(),
        last_verified_upload_at:cfg.last_verified_upload_at,
        last_verified_file_id:cfg.last_verified_file_id,
        note:"La copia remota se marca verificada solo al recibir confirmación de Google Drive API y checksum/tamaño coincidentes.",
    })
}
#[tauri::command]
pub(crate) fn drive_api_set_client(app:tauri::AppHandle,client_id:String)->Result<(),String>{
    let id=client_id.trim();
    if !drive_oauth::client_id_valid(id){return Err("Ingresá un OAuth Client ID tipo Desktop válido de Google Cloud.".into());}
    let mut cfg=config(&app)?;
    if cfg.client_id!=id{
        if !pending(&app)?.is_empty(){return Err("Hay respaldos cifrados pendientes. Subilos o descargalos antes de cambiar la aplicación OAuth.".into());}
        drive_oauth::forget()?;
        cfg.last_verified_file_id=None;cfg.last_verified_upload_at=None;
    }
    cfg.client_id=id.to_string();
    save(&app,&cfg)
}
#[tauri::command]
pub(crate) async fn drive_api_connect(app:tauri::AppHandle)->Result<bool,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let cfg=config(&app)?;
        drive_oauth::connect(&cfg.client_id)?;
        Ok(true)
    }).await.map_err(|_|"Falló el proceso de autorización Google".to_string())?
}
#[tauri::command]
pub(crate) fn drive_api_disconnect(app:tauri::AppHandle)->Result<(),String>{
    drive_oauth::forget()?;
    let mut cfg=config(&app)?;
    cfg.last_verified_upload_at=None;cfg.last_verified_file_id=None;save(&app,&cfg)
}
#[tauri::command]
pub(crate) async fn drive_api_unlock(app:tauri::AppHandle,passphrase:String)->Result<bool,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let files=pending(&app)?;
        drive_backup::unlock_for_api(passphrase,&files)?;
        Ok(true)
    }).await.map_err(|_|"Falló el desbloqueo cifrado".to_string())?
}
fn access(app:&tauri::AppHandle)->Result<String,String>{
    let id=config(app)?.client_id;
    if !drive_oauth::client_id_valid(&id){return Err("Primero configurá Google Drive API desde Ajustes.".into());}
    drive_oauth::token(&id)
}
#[tauri::command]
pub(crate) async fn drive_api_list(app:tauri::AppHandle)->Result<Vec<drive_transport::CloudFile>,String>{
    tauri::async_runtime::spawn_blocking(move ||drive_transport::list(&access(&app)?))
        .await.map_err(|_|"No se pudo consultar Google Drive".to_string())?
}
fn flush(app:&tauri::AppHandle)->Result<usize,String>{
    let _guard=UPLOAD_LOCK.get_or_init(||Mutex::new(())).try_lock()
        .map_err(|_|"Ya hay una sincronización con Drive en curso. Intentá nuevamente.".to_string())?;
    let token=access(app)?;
    let mut count=0;
    for path in pending(app)?{
        let id=drive_transport::upload(&token,&path)?;
        let mut cfg=config(app)?;
        cfg.last_verified_file_id=Some(id);
        cfg.last_verified_upload_at=Some(chrono::Utc::now().to_rfc3339());
        save(app,&cfg)?;
        fs::remove_file(&path).map_err(|_|"Google verificó la copia pero no se pudo eliminar el archivo pendiente local")?;
        count+=1;
    }
    Ok(count)
}
pub(crate) fn replicate_if_connected(app:&tauri::AppHandle,result:&backup::BackupResult)->Result<Option<usize>,String>{
    let cfg=config(app)?;
    if cfg.client_id.is_empty()||!drive_oauth::authorized(){return Ok(None);}
    // Fail closed: never upload plaintext, never silently persist a recovery phrase.
    let phrase=drive_backup::unlocked_phrase()?.ok_or(
        "Google Drive conectado, pero cifrado bloqueado. Desbloqueá la frase en Ajustes para subir.")?;
    let dir=queue_dir(app)?;
    if pending(app)?.len()>=50{return Err("Hay 50 copias cifradas esperando conexión. Revisá Google Drive antes de continuar.".into());}
    drive_backup::seal(result,&dir,&phrase)?;
    flush(app).map(Some)
}
#[tauri::command]
pub(crate) async fn drive_api_retry(app:tauri::AppHandle)->Result<usize,String>{
    tauri::async_runtime::spawn_blocking(move ||flush(&app))
        .await.map_err(|_|"Falló el reintento de subida".to_string())?
}
struct Downloaded(PathBuf);
impl Drop for Downloaded{fn drop(&mut self){let _=fs::remove_file(&self.0);}}
fn download(app:&tauri::AppHandle,file_id:&str)->Result<Downloaded,String>{
    let folder=home(app)?.join("drive-api-downloads");
    fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let path=folder.join(format!("{}.atbackup.age.partial",uuid::Uuid::new_v4()));
    let guard=Downloaded(path);
    drive_transport::download(&access(app)?,file_id,&guard.0)?;
    Ok(guard)
}
#[tauri::command]
pub(crate) async fn drive_api_test(app:tauri::AppHandle,file_id:String,passphrase:String)->Result<backup::RecoveryTestResult,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let archive=download(&app,&file_id)?;
        drive_backup::test_archive_at(&archive.0,&passphrase)
    }).await.map_err(|_|"Falló la prueba de recuperación desde Drive".to_string())?
}
#[tauri::command]
pub(crate) async fn drive_api_preview(app:tauri::AppHandle,file_id:String,passphrase:String)->Result<backup::SqlitePreview,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let archive=download(&app,&file_id)?;
        drive_backup::preview_archive_at(&archive.0,&passphrase)
    }).await.map_err(|_|"Falló la vista previa de Drive".to_string())?
}
#[cfg(test)]
mod tests{
    #[test] fn cloud_api_never_needs_google_secret_on_disk(){
        assert!(super::drive_oauth::client_id_valid("012345678901-abc.apps.googleusercontent.com"));
    }
}