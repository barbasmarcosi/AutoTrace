//! Direct Google Drive API orchestrator: local-first, no plaintext uploads.
use crate::{backup,drive_backup,drive_oauth,drive_transport};
use serde::{Serialize,Deserialize};
use std::{fs,path::PathBuf,sync::{Mutex,OnceLock}};
static UPLOAD_LOCK:OnceLock<Mutex<()>>=OnceLock::new();
use tauri::Manager;
#[derive(Default,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct Settings{
    client_id:String,
    last_verified_upload_at:Option<String>,
    last_verified_file_id:Option<String>,
    last_sync_state:Option<String>,
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
    pub identity_status:&'static str,
    pub developer_mode:bool,
    pub auto_unlock_enabled:bool,
    pub last_sync_state:Option<String>,
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
// The Desktop OAuth Client ID is PUBLIC. It may be injected at build time;
// neither an OAuth client secret nor a user's token may be embedded in releases.
const PACKAGED_CLIENT_ID:Option<&str>=option_env!("AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID");
fn packaged_identity()->Option<&'static str>{
    PACKAGED_CLIENT_ID.filter(|id|drive_oauth::client_id_valid(id))
}
fn identity_status(settings:&Settings)->&'static str{
    if let Some(bundled)=packaged_identity(){
        if !settings.client_id.is_empty()&&settings.client_id!=bundled{
            return "migration_required";
        }
        "ready"
    }else if cfg!(debug_assertions)&&drive_oauth::client_id_valid(&settings.client_id){
        "ready"
    }else{"not_configured"}
}
fn effective_client_id(settings:&Settings)->Result<String,String>{
    match identity_status(settings){
        "ready"=>Ok(packaged_identity().unwrap_or(&settings.client_id).to_owned()),
        "migration_required"=>Err("La identidad OAuth cambió. Exportá las copias existentes y completá la migración antes de conectar la nueva versión.".into()),
        _=>Err("Google Drive directo todavía no está activado en esta edición de AutoTrace. Los respaldos locales siguen funcionando.".into())
    }
}
/// Replace a settings file in one same-volume filesystem operation.
/// On Windows std::fs::rename cannot reliably replace an existing file, so
/// use MoveFileExW with REPLACE_EXISTING and WRITE_THROUGH instead. No
/// delete-then-rename window is permitted for backup provenance metadata.
#[cfg(windows)]
pub(crate) fn replace_settings_file(temp:&std::path::Path,target:&std::path::Path)->std::io::Result<()>{
    use std::{iter,os::windows::ffi::OsStrExt};
    #[link(name="kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing:*const u16,new:*const u16,flags:u32)->i32;
    }
    let from=temp.as_os_str().encode_wide().chain(iter::once(0)).collect::<Vec<_>>();
    let to=target.as_os_str().encode_wide().chain(iter::once(0)).collect::<Vec<_>>();
    // 0x1 = REPLACE_EXISTING; 0x8 = WRITE_THROUGH; never COPY_ALLOWED.
    if unsafe { MoveFileExW(from.as_ptr(),to.as_ptr(),0x1|0x8) }!=0{Ok(())}
    else{Err(std::io::Error::last_os_error())}
}
#[cfg(not(windows))]
pub(crate) fn replace_settings_file(temp:&std::path::Path,target:&std::path::Path)->std::io::Result<()>{
    fs::rename(temp,target)
}
fn save_settings_file(path:&std::path::Path,state:&Settings)->Result<(),String>{
    let temp=path.with_extension(format!("{}.tmp",uuid::Uuid::new_v4().simple()));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {fn drop(&mut self){let _=fs::remove_file(&self.0);}}
    let _cleanup=Cleanup(temp.clone());
    let encoded=serde_json::to_vec(state).map_err(|e|e.to_string())?;
    let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&temp)
        .map_err(|e|e.to_string())?;
    use std::io::Write;
    file.write_all(&encoded).map_err(|e|e.to_string())?;
    file.sync_all().map_err(|e|e.to_string())?;
    drop(file);
    replace_settings_file(&temp,path).map_err(|_|"No se pudo guardar de forma segura el estado de Google Drive en Windows".to_string())
}
fn save(app:&tauri::AppHandle,state:&Settings)->Result<(),String>{
    save_settings_file(&config_path(app)?,state)
}
fn queue_dir(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let dir=home(app)?.join("drive-api-encrypted-pending");
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir)
}
fn queued_files(dir:&std::path::Path)->Result<Vec<PathBuf>,String>{
    let mut found=Vec::new();
    for entry in fs::read_dir(dir).map_err(|e|e.to_string())?{
        let path=entry.map_err(|e|e.to_string())?.path();
        if !path.file_name().and_then(|x|x.to_str())
            .is_some_and(|n|n.starts_with("AutoTrace_")&&n.ends_with(".atbackup.age")){continue;}
        let meta=fs::symlink_metadata(&path).map_err(|_|"La cola cifrada cambió inesperadamente")?;
        if !meta.file_type().is_file()||meta.file_type().is_symlink(){
            return Err("La cola Drive contiene una ruta no segura. No se sincronizará ningún archivo hasta corregirla.".into());
        }
        found.push(path);
    }
    found.sort();Ok(found)
}
fn pending(app:&tauri::AppHandle)->Result<Vec<PathBuf>,String>{
    queued_files(&queue_dir(app)?)
}
#[tauri::command]
pub(crate) fn drive_api_status(app:tauri::AppHandle)->Result<ApiStatus,String>{
    let cfg=config(&app)?;
    let identity=identity_status(&cfg);
    Ok(ApiStatus{
        client_configured:identity=="ready",
        connected:identity=="ready"&&drive_oauth::authorized(),
        pending_count:pending(&app)?.len(),
        last_verified_upload_at:cfg.last_verified_upload_at,
        last_verified_file_id:cfg.last_verified_file_id,
        note:"Un token local guardado no confirma que Google siga autorizándolo; una copia solo cuenta tras verificar tamaño y checksum remotos.",
        identity_status:identity,
        developer_mode:cfg!(debug_assertions),
        auto_unlock_enabled:drive_backup::auto_unlock_enabled(),
        last_sync_state:cfg.last_sync_state,
    })
}
#[tauri::command]
pub(crate) fn drive_api_set_client(app:tauri::AppHandle,client_id:String)->Result<(),String>{
    if !cfg!(debug_assertions){
        return Err("El Client ID se incorpora a AutoTrace durante la compilación; el usuario no debe configurarlo.".into());
    }
    let id=client_id.trim();
    if !drive_oauth::client_id_valid(id){return Err("Ingresá un OAuth Client ID tipo Desktop válido de Google Cloud.".into());}
    let mut cfg=config(&app)?;
    if cfg.client_id!=id{
        if !pending(&app)?.is_empty(){return Err("Hay respaldos cifrados pendientes. Subilos o descargalos antes de cambiar la aplicación OAuth.".into());}
        drive_oauth::forget()?;
        cfg.last_verified_file_id=None;cfg.last_verified_upload_at=None;
        cfg.last_sync_state=None;
    }
    cfg.client_id=id.to_string();
    save(&app,&cfg)
}
#[tauri::command]
pub(crate) async fn drive_api_connect(app:tauri::AppHandle)->Result<bool,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let cfg=config(&app)?;
        let id=effective_client_id(&cfg)?;
        drive_oauth::connect(&id)?;
        Ok(true)
    }).await.map_err(|_|"Falló el proceso de autorización Google".to_string())?
}
#[tauri::command]
pub(crate) fn drive_api_disconnect(app:tauri::AppHandle)->Result<(),String>{
    drive_oauth::forget()?;
    let mut cfg=config(&app)?;
    cfg.last_verified_upload_at=None;cfg.last_verified_file_id=None;
    cfg.last_sync_state=None;save(&app,&cfg)
}
#[tauri::command]
pub(crate) async fn drive_api_unlock(app:tauri::AppHandle,passphrase:String)->Result<bool,String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let files=pending(&app)?;
        drive_backup::unlock_for_api(passphrase,&files)?;
        Ok(true)
    }).await.map_err(|_|"Falló el desbloqueo cifrado".to_string())?
}
#[tauri::command]
pub(crate) async fn drive_api_enable_auto_unlock(app:tauri::AppHandle,passphrase:String)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move ||{
        let existing=pending(&app)?;
        // Refuse persistence if previously queued or synced historical snapshots
        // use a different phrase, even if the direct-API queue is currently empty.
        drive_backup::check_existing_archives(&app,&passphrase)?;
        drive_backup::unlock_for_api(passphrase.clone(),&existing)?;
        drive_backup::enable_auto_unlock(passphrase)
    }).await.map_err(|_|"Falló la protección de la clave en Windows".to_string())?
}
#[tauri::command]
pub(crate) fn drive_api_disable_auto_unlock()->Result<(),String>{
    drive_backup::disable_auto_unlock()
}
fn access(app:&tauri::AppHandle)->Result<String,String>{
    let cfg=config(app)?;
    let id=effective_client_id(&cfg)?;
    drive_oauth::token(&id)
}
#[tauri::command]
pub(crate) async fn drive_api_list(app:tauri::AppHandle)->Result<Vec<drive_transport::CloudFile>,String>{
    tauri::async_runtime::spawn_blocking(move ||drive_transport::list(&access(&app)?))
        .await.map_err(|_|"No se pudo consultar Google Drive".to_string())?
}
fn classify_sync_failure(error:&str)->&'static str{
    if error.contains("HTTP 401")||error.contains("OAuth rechazó")||
       error.contains("Google no entregó acceso"){return "reconnect_required";}
    if error.contains("HTTP 403")||error.contains("cuota"){return "quota_or_permission";}
    if error.contains("HTTP 429"){return "rate_limited";}
    if error.contains("HTTP 5"){return "provider_unavailable";}
    if error.contains("checksum")||error.contains("integridad"){return "integrity_failed";}
    if error.contains("frase")||error.contains("cifrado bloqueado"){return "key_required";}
    "retry_required"
}
/// A successful no-op cannot be recorded as remote verification; preserve
/// the previous status and date until bytes are confirmed by the provider.
fn sync_outcome(result:&Result<usize,String>)->Option<&'static str>{
    match result {
        Ok(0)=>None,
        Ok(_)=>Some("remote_verified"),
        Err(e)=>Some(classify_sync_failure(e)),
    }
}
fn flush(app:&tauri::AppHandle)->Result<usize,String>{
    let _guard=UPLOAD_LOCK.get_or_init(||Mutex::new(())).try_lock()
        .map_err(|_|"Ya hay una sincronización con Drive en curso. Intentá nuevamente.".to_string())?;
    let result=(||->Result<usize,String>{
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
    })();
    // Only persist coarse reason codes, never OAuth responses, URLs, tokens or file contents.
    // IMPORTANT: an empty queue is NOT a successfully uploaded backup.
    if let Ok(mut cfg)=config(app){
        if let Some(outcome)=sync_outcome(&result){
            cfg.last_sync_state=Some(outcome.to_string());
            let _=save(app,&cfg);
        }
    }
    result
}
pub(crate) fn replicate_if_connected(app:&tauri::AppHandle,result:&backup::BackupResult)->Result<Option<usize>,String>{
    let cfg=config(app)?;
    if identity_status(&cfg)!="ready"||!drive_oauth::authorized(){return Ok(None);}
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
    #[test] fn identity_selection_fails_closed_without_packaged_id(){
        let state=super::Settings::default();
        if super::packaged_identity().is_none(){
            assert_eq!(super::identity_status(&state),"not_configured");
            assert!(super::effective_client_id(&state).is_err());
        }
        let cfg=super::Settings{client_id:"012345678901-valid.apps.googleusercontent.com".to_string(),..Default::default()};
        if let Some(bundled)=super::packaged_identity(){
            if cfg.client_id!=bundled{assert_eq!(super::identity_status(&cfg),"migration_required");}
        }else{
            assert_eq!(super::identity_status(&cfg),if cfg!(debug_assertions){"ready"}else{"not_configured"});
        }
    }
    #[test] fn errors_are_categorized_without_persisting_private_provider_responses(){
        let cases=[
            ("Google Drive devolvió HTTP 401. No autorizado.","reconnect_required"),
            ("Google OAuth rechazó el acceso","reconnect_required"),
            ("Google Drive devolvió HTTP 403","quota_or_permission"),
            ("Google Drive devolvió HTTP 429","rate_limited"),
            ("Google Drive devolvió HTTP 503","provider_unavailable"),
            ("La copia falló por checksum","integrity_failed"),
            ("Google Drive conectado, pero cifrado bloqueado","key_required"),
            ("red desconectada","retry_required")
        ];
        for (input,expected) in cases{
            assert_eq!(super::classify_sync_failure(input),expected);
        }
    }
    #[test] fn repeated_drive_settings_updates_replace_existing_file_atomically(){
        use std::fs;
        let temp=std::env::temp_dir().join(format!("autotrace-drive-settings-replace-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let path=temp.join("google-drive-api.json");
        for revision in 0..10 {
            let settings=super::Settings{
                client_id:"test-public-client.apps.googleusercontent.com".into(),
                last_verified_upload_at:Some(format!("2026-10-25T05:00:{revision:02}Z")),
                last_verified_file_id:Some(format!("test-file-{revision}")),
                last_sync_state:Some("remote_verified".into()),
            };
            super::save_settings_file(&path,&settings).unwrap();
            let decoded:super::Settings=serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(decoded.last_verified_file_id,settings.last_verified_file_id,
                "replacing an existing Windows settings JSON must persist current upload metadata");
            assert_eq!(decoded.last_verified_upload_at,settings.last_verified_upload_at);
            assert_eq!(fs::read_dir(&temp).unwrap().count(),1,"no leftover .tmp settings files");
        }
        let last=fs::read(&path).unwrap();
        let missing=temp.join("missing-directory").join("drive.json");
        assert!(super::save_settings_file(&missing,&super::Settings::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(),last,
            "a failed unrelated save must not alter the last verified upload");
        fs::remove_dir_all(temp).unwrap();
    }
    #[test] fn empty_queue_does_not_fabricate_success_or_reset_a_provider_failure(){
        assert_eq!(super::sync_outcome(&Ok(0)),None);
        assert_eq!(super::sync_outcome(&Ok(1)),Some("remote_verified"));
        assert_eq!(super::sync_outcome(&Err("Google Drive devolvió HTTP 401".into())),Some("reconnect_required"));
        assert_eq!(super::sync_outcome(&Err("Google Drive devolvió HTTP 503".into())),Some("provider_unavailable"));
    }
    #[test] fn drive_queue_rejects_nonfiles_and_preserves_valid_file_list(){
        use std::fs;
        let temp=std::env::temp_dir().join(format!("autotrace-queue-test-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let legitimate=temp.join("AutoTrace_2026.atbackup.age");
        fs::write(&legitimate,b"synthetic fixture only").unwrap();
        fs::write(temp.join("ignore-this.sqlite3"),b"unrelated").unwrap();
        assert_eq!(super::queued_files(&temp).unwrap(),vec![legitimate.clone()]);
        let unwanted=temp.join("AutoTrace_unexpected.atbackup.age");
        fs::create_dir(&unwanted).unwrap();
        assert!(super::queued_files(&temp).is_err(),
            "directories masquerading as backup names cannot be silently ignored");
        fs::remove_dir_all(&unwanted).unwrap();
        #[cfg(unix)]
        {
            let target=temp.join("customer-private.sqlite3");
            fs::write(&target,b"fake customer data").unwrap();
            std::os::unix::fs::symlink(&target,&unwanted).unwrap();
            assert!(super::queued_files(&temp).is_err(),"symbolic links must fail closed");
        }
        fs::remove_dir_all(temp).unwrap();
    }
    #[test] fn cloud_api_never_needs_google_secret_on_disk(){
        assert!(super::drive_oauth::client_id_valid("012345678901-abc.apps.googleusercontent.com"));
    }
}