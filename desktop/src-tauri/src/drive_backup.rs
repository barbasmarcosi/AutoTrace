//! Encrypted off-device replication through a user-chosen Google Drive for desktop folder.
//! This module does NOT call Google APIs, store a password, or assert that Drive synced.
use crate::backup;
use age::secrecy::SecretString;
use serde::Serialize;
use std::{fs,io::{Read,Write,BufReader,BufWriter},path::{Path,PathBuf},sync::{Mutex,OnceLock},time::UNIX_EPOCH};
use tauri::Manager;

const MAGIC: &[u8] = b"AUTOTRACE-DRIVE-1\n";
const MAX_SQLITE: u64 = 512_000_000;
const MAX_EXCEL: u64 = 25_000_000;
const MAX_MANIFEST: u64 = 16_384;
const MIN_PASSPHRASE_CHARS: usize = 16;
static UNLOCKED: OnceLock<Mutex<Option<SecretString>>> = OnceLock::new();
const WINDOWS_KEY_SERVICE:&str="AutoTrace-Backup-Age";
const WINDOWS_KEY_ACCOUNT:&str="local-auto-unlock";
fn windows_key()->Result<keyring::Entry,String>{
    keyring::Entry::new(WINDOWS_KEY_SERVICE,WINDOWS_KEY_ACCOUNT)
      .map_err(|_|"El almacén seguro de claves de Windows no está disponible.".into())
}
pub(crate) fn auto_unlock_enabled()->bool{
    windows_key().is_ok_and(|entry|entry.get_password().is_ok())
}
pub(crate) fn phrase_for_backup()->Result<Option<SecretString>,String>{
    if let Some(in_session)=active_secret()?{return Ok(Some(in_session));}
    match windows_key()?.get_password(){
        Ok(value)=>{
            check_pass(&value)?;
            Ok(Some(SecretString::from(value)))
        },
        Err(keyring::Error::NoEntry)=>Ok(None),
        Err(_)=>Err("No se pudo desbloquear la clave protegida por Windows. Usá tu frase de recuperación en Ajustes.".into())
    }
}
/// Reusing a different recovery phrase would make the user's historical
/// encrypted archives unreadable with the recorded recovery kit.
fn ensure_phrase_consistent(passphrase:&str)->Result<(),String>{
    check_pass(passphrase)?;
    match windows_key()?.get_password(){
        Ok(existing) if existing!=passphrase=>
            return Err("Esta frase es distinta de la clave ya protegida en Windows. Para cambiarla se necesita un proceso explícito de rotación, no sobrescribirla.".into()),
        Ok(_)|Err(keyring::Error::NoEntry)=>{},
        Err(_)=>return Err("No se pudo comprobar la clave protegida por Windows. No se modificó el kit de recuperación.".into()),
    }
    if let Some(active)=active_secret()?{
        use age::secrecy::ExposeSecret;
        if active.expose_secret()!=passphrase {
            return Err("La frase no coincide con la clave desbloqueada en esta sesión. No se mezclaron respaldos con claves diferentes.".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn enable_auto_unlock(passphrase:String)->Result<(),String>{
    ensure_phrase_consistent(&passphrase)?;
    // This is an explicit opt-in. Windows Credential Manager protects the secret
    // for the current Windows account; the portable phrase is still needed on a new PC.
    windows_key()?.set_password(&passphrase)
      .map_err(|_|"Windows no pudo proteger la clave de respaldos.".to_string())?;
    set_secret(Some(SecretString::from(passphrase)))
}
/// 244 bits of OS-backed UUIDv4 randomness across two independent values.
/// The phrase is returned ONCE to the Desktop recovery-kit dialog; never logged or saved.
#[tauri::command]
pub(crate) fn generate_recovery_phrase()->String{
    format!("AT-{}-{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple())
}
pub(crate) fn disable_auto_unlock()->Result<(),String>{
    let key=windows_key()?;
    match key.delete_credential(){
        Ok(())|Err(keyring::Error::NoEntry)=>{},
        Err(_)=>return Err("Windows no pudo eliminar la clave automática.".into())
    }
    set_secret(None)
}

fn active_secret() -> Result<Option<SecretString>,String> {
    let guard=UNLOCKED.get_or_init(||Mutex::new(None)).lock().map_err(|_|"Bloqueo de clave no disponible")?;
    Ok(guard.clone())
}
fn set_secret(pass:Option<SecretString>)->Result<(),String>{
    let mut guard=UNLOCKED.get_or_init(||Mutex::new(None)).lock().map_err(|_|"Bloqueo de clave no disponible")?;
    *guard=pass;
    Ok(())
}
fn check_pass(pass:&str)->Result<(),String>{
    if pass.chars().count()<MIN_PASSPHRASE_CHARS||pass.chars().count()>256 {
        return Err("La frase de recuperación debe tener entre 16 y 256 caracteres.".into());
    }
    Ok(())
}
fn config_path(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let root=app.path().app_local_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    Ok(root.join("drive-encrypted-backup.json"))
}
fn configured_directory(app:&tauri::AppHandle)->Result<Option<PathBuf>,String>{
    let path=config_path(app)?;
    if !path.exists(){return Ok(None);}
    let doc:serde_json::Value=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let directory=doc["directory"].as_str().ok_or("La carpeta cifrada configurada es inválida")?;
    if directory.is_empty(){return Err("Carpeta cifrada no configurada".into());}
    Ok(Some(PathBuf::from(directory)))
}
fn paths_overlap(a:&Path,b:&Path)->bool{a.starts_with(b)||b.starts_with(a)}
pub(crate) fn overlaps_encrypted_destination(app:&tauri::AppHandle,folder:&Path)->Result<bool,String>{
    let Some(cloud)=configured_directory(app)? else {return Ok(false)};
    let local=fs::canonicalize(folder).unwrap_or_else(|_|folder.to_path_buf());
    let cloud=fs::canonicalize(&cloud).unwrap_or(cloud);
    Ok(paths_overlap(&local,&cloud))
}
/// Validate that an existing synced folder is compatible with the phrase
/// BEFORE storing a new auto-unlock credential. This protects historical copies
/// even when Google's direct-API pending queue is empty.
pub(crate) fn check_existing_archives(app:&tauri::AppHandle,passphrase:&str)->Result<(),String>{
    ensure_phrase_consistent(passphrase)?;
    if let Some(folder)=configured_directory(app)?{
        let archives=archives_in(&folder)?;
        verify_synced_archive_keys(&archives,passphrase)?;
        // Full content-integrity checks for both oldest and newest snapshots.
        // Middle archives are independently verified on actual restore.
        if let Some(oldest)=archives.first(){let _=read_archive(oldest,passphrase)?;}
        if let Some(newest)=archives.last(){
            if archives.first()!=Some(newest){let _=read_archive(newest,passphrase)?;}
        }
    }
    Ok(())
}
fn is_plain_workshop_backup_name(name:&str)->bool{
    name.starts_with("AutoTrace_")&&[".sqlite3",".xlsx",".manifest.json"].iter().any(|ext|name.ends_with(ext))
}
fn reject_plaintext_in_sync_folder(folder:&Path)->Result<(),String>{
    for entry in fs::read_dir(folder).map_err(|e|e.to_string())?{
        let filename=entry.map_err(|e|e.to_string())?.file_name();
        if filename.to_str().is_some_and(is_plain_workshop_backup_name){
            return Err("Hay respaldos AutoTrace SIN CIFRAR en la carpeta elegida. Movelos fuera de Google Drive antes de activar esta función.".into());
        }
    }
    Ok(())
}
fn archives_in(folder:&Path)->Result<Vec<PathBuf>,String>{
    if !folder.is_dir(){return Err("Carpeta sincronizada no disponible".into());}
    let mut paths=Vec::new();
    for entry in fs::read_dir(folder).map_err(|e|e.to_string())? {
        let path=entry.map_err(|e|e.to_string())?.path();
        let Some(name)=path.file_name().and_then(|s|s.to_str()) else {continue};
        if name.starts_with("AutoTrace_")&&name.ends_with(".atbackup.age")&&path.is_file(){paths.push(path);}
    }
    paths.sort();
    Ok(paths)
}
/// A folder may contain several generations of encrypted backups. Testing
/// only its newest file can silently accept a history mixed across different
/// recovery phrases. Verify the authenticated beginning of *every* archive;
/// avoid decrypting all large SQLite/Excel bodies just to check their keys.
/// Full integrity is still checked separately for the endpoints and at restore.
fn verify_synced_archive_keys(paths:&[PathBuf],passphrase:&str)->Result<(),String>{
    for path in paths {
        verify_archive_key(path,passphrase).map_err(|_|
            "Al menos una copia cifrada de la carpeta no coincide con la frase o tiene la cabecera dañada. No se configuró ni desbloqueó el historial.".to_string())?;
    }
    Ok(())
}
fn verify_archive_key(path:&Path,passphrase:&str)->Result<(),String>{
    let meta=fs::metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.len()<100||meta.len()>600_000_000{
        return Err("Archivo cifrado fuera de tamaño permitido".into());
    }
    let decryptor=age::Decryptor::new(BufReader::new(fs::File::open(path).map_err(|e|e.to_string())?))
        .map_err(|_|"Archivo cifrado sin encabezado age válido")?;
    let identity=age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));
    let mut reader=decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_|"Frase de recuperación incompatible")?;
    let mut magic=vec![0u8;MAGIC.len()];
    reader.read_exact(&mut magic).map_err(|_|"Cabecera cifrada incompleta")?;
    if magic!=MAGIC{return Err("Formato de archivo cifrado incompatible".into());}
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct DriveStatus {
    pub directory:Option<String>,
    pub unlocked:bool,
    pub last_encrypted_file_unix_ms:Option<u128>,
    pub sync_verified:bool,
    pub note:&'static str,
}
#[tauri::command]
pub(crate) fn drive_backup_status(app:tauri::AppHandle)->Result<DriveStatus,String>{
    let directory=configured_directory(&app)?;
    let last=if let Some(dir)=directory.as_ref(){
        if !dir.exists(){None}else{
            archives_in(dir)?.into_iter().filter_map(|p|fs::metadata(p).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d|d.as_millis())).max()
        }
    }else{None};
    Ok(DriveStatus{
        directory:directory.map(|d|d.display().to_string()),
        unlocked:active_secret()?.is_some(),
        last_encrypted_file_unix_ms:last,
        sync_verified:false,
        note:"Archivo cifrado creado localmente; comprobá en Google Drive que terminó de sincronizar.",
    })
}
#[tauri::command]
pub(crate) fn configure_drive_backup(app:tauri::AppHandle,passphrase:String)->Result<Option<String>,String>{
    check_pass(&passphrase)?;
    let Some(picked)=rfd::FileDialog::new()
      .set_title("Seleccioná una carpeta LOCAL sincronizada por Google Drive para escritorio")
      .pick_folder() else {return Ok(None)};
    let directory=fs::canonicalize(picked).map_err(|e|e.to_string())?;
    let local=app.path().app_local_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&local).map_err(|e|e.to_string())?;
    let local=fs::canonicalize(local).map_err(|e|e.to_string())?;
    if directory.starts_with(&local)||local.starts_with(&directory){
        return Err("No se permite usar la carpeta de datos originales como destino cifrado.".into());
    }
    // Prevent accidental replication of UNENCRYPTED external backups into the same Drive tree.
    if let Some(external)=backup::configured_folder(&app)? {
        let external=fs::canonicalize(&external).unwrap_or(external);
        if paths_overlap(&external,&directory){
            return Err("Ya existe una copia externa sin cifrar en esta carpeta o una carpeta superior. Elegí otro destino seguro para no subir SQLite/Excel sin cifrar.".into());
        }
    }
    // Do not activate this destination if old cleartext backups already exist here.
    reject_plaintext_in_sync_folder(&directory)?;
    // An already-populated destination cannot silently start using a different key.
    let existing=archives_in(&directory)?;
    verify_synced_archive_keys(&existing,&passphrase)?;
    // Preserve the deep end-to-end checksum check on the freshest file.
    if let Some(first)=existing.last(){
        let _=read_archive(first,&passphrase)?;
    }
    let cfg=config_path(&app)?;
    let staged=cfg.with_extension("json.tmp");
    let payload=serde_json::to_vec(&serde_json::json!({"directory":directory})).map_err(|e|e.to_string())?;
    fs::write(&staged,payload).map_err(|e|e.to_string())?;
    if let Err(e)=fs::rename(&staged,&cfg){let _=fs::remove_file(staged);return Err(e.to_string());}
    set_secret(Some(SecretString::from(passphrase)))?;
    Ok(Some(directory.display().to_string()))
}
#[tauri::command]
pub(crate) fn unlock_drive_backup(app:tauri::AppHandle,passphrase:String)->Result<bool,String>{
    check_pass(&passphrase)?;
    let dir=configured_directory(&app)?.ok_or("Configurá primero una carpeta cifrada.")?;
    // Verify decryptability and inner SQLite/Excel checksums BEFORE accepting the key.
    let history=archives_in(&dir)?;
    verify_synced_archive_keys(&history,&passphrase)?;
    if let Some(first)=history.last(){let _=read_archive(first,&passphrase)?;}
    set_secret(Some(SecretString::from(passphrase)))?;
    Ok(true)
}
#[tauri::command]
pub(crate) fn lock_drive_backup()->Result<(),String>{set_secret(None)}
pub(crate) fn unlocked_phrase()->Result<Option<SecretString>,String>{phrase_for_backup()}
/// A queued cloud backup may outlive the current process. Never authorize a
/// new recovery phrase by testing only the first of several queued archives.
/// The API writer limits pending backups to 50: refuse an unexpectedly larger
/// directory rather than silently sampling and misrepresenting its safety.
fn verify_pending_archives(files:&[PathBuf],passphrase:&str)->Result<(),String>{
    if files.len()>50{
        return Err("Hay más de 50 respaldos cifrados pendientes. Revisá la cola antes de desbloquearla.".into());
    }
    for file in files{
        let _=read_archive(file,passphrase).map_err(|_|
            "Al menos un respaldo cifrado pendiente no puede recuperarse con esta frase. No se desbloqueó la cola.".to_string())?;
    }
    Ok(())
}
pub(crate) fn unlock_for_api(passphrase:String,files:&[PathBuf])->Result<(),String>{
    ensure_phrase_consistent(&passphrase)?;
    verify_pending_archives(files,&passphrase)?;
    set_secret(Some(SecretString::from(passphrase)))
}
/// Export a fresh, fully verified encrypted disaster-recovery snapshot to
/// a user-selected file. No Google OAuth, Google Desktop or web access required.
#[tauri::command]
pub(crate) async fn export_portable_encrypted_backup(app:tauri::AppHandle,passphrase:String)->Result<Option<String>,String>{
    check_pass(&passphrase)?;
    let Some(destination)=rfd::FileDialog::new()
        .set_title("Guardá esta copia CIFRADA en un pendrive o disco diferente")
        .add_filter("Respaldo cifrado AutoTrace",&["age"])
        .set_file_name("AutoTrace_respaldo_portatil.atbackup.age")
        .save_file() else {return Ok(None)};
    let filename=destination.file_name().map(|n|n.to_string_lossy().into_owned());
    tauri::async_runtime::spawn_blocking(move ||{
        export_portable_snapshot(&app,&destination,&passphrase)
    }).await.map_err(|_|"Falló el proceso de exportación cifrada".to_owned())??;
    Ok(filename)
}
fn export_portable_snapshot(app:&tauri::AppHandle,destination:&Path,phrase:&str)->Result<(),String>{
    check_pass(phrase)?;
    if destination.exists(){return Err("El archivo de destino ya existe: elegí otro nombre para no sobrescribir respaldos.".into());}
    let local=app.path().app_local_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&local).map_err(|e|e.to_string())?;
    let local=fs::canonicalize(&local).map_err(|e|e.to_string())?;
    let parent=destination.parent().ok_or("Destino de exportación inválido")?;
    let parent=fs::canonicalize(parent).map_err(|e|e.to_string())?;
    // A local file on the same volume does not protect against failure of the PC.
    backup::check_distinct_volume(&local,&parent)?;
    ensure_phrase_consistent(phrase)?;
    let private=local.join("backups");
    fs::create_dir_all(&private).map_err(|e|e.to_string())?;
    // Use a normal timestamped snapshot ID so the local dual pair remains
    // eligible for the conservative retention policy instead of accumulating forever.
    let id=format!("{}-{}",chrono::Utc::now().timestamp_millis(),uuid::Uuid::new_v4().simple());
    let pair=backup::backup_pair(&crate::store::open_app(app)?,&private,&id)?;
    let cipher_dir=local.join("encrypted-export-staging");
    fs::create_dir_all(&cipher_dir).map_err(|e|e.to_string())?;
    let cipher=seal(&pair,&cipher_dir,&SecretString::from(phrase.to_owned()))?;
    struct DeleteOnDrop(PathBuf);
    impl Drop for DeleteOnDrop { fn drop(&mut self){let _=fs::remove_file(&self.0);} }
    let _cipher_cleanup=DeleteOnDrop(cipher.clone());
    copy_verified_cipher(&cipher,destination)
}
/// Copy only already-encrypted bytes. Publish the final name after a durable,
/// checksum-verified copy. Failure leaves no new final file.
fn copy_verified_cipher(cipher:&Path,destination:&Path)->Result<(),String>{
    if destination.exists(){return Err("El archivo de destino ya existe: no se sobrescribió ningún respaldo.".into());}
    let parent=destination.parent().ok_or("Destino de exportación inválido")?;
    struct DeleteOnDrop(PathBuf);
    impl Drop for DeleteOnDrop {fn drop(&mut self){let _=fs::remove_file(&self.0);}}
    let temp=parent.join(format!(".autotrace-export-{}.partial",uuid::Uuid::new_v4()));
    let _temp_cleanup=DeleteOnDrop(temp.clone());
    let mut from=fs::File::open(cipher).map_err(|e|e.to_string())?;
    let mut out=fs::File::create_new(&temp).map_err(|e|e.to_string())?;
    std::io::copy(&mut from,&mut out).map_err(|e|e.to_string())?;
    out.sync_all().map_err(|e|e.to_string())?;
    drop(out);
    // Verify that the destination copy contains exactly the authenticated ciphertext.
    fn hash(path:&Path)->Result<Vec<u8>,String>{
        use sha2::{Digest,Sha256};
        let mut h=Sha256::new();let mut f=fs::File::open(path).map_err(|e|e.to_string())?;
        let mut chunk=[0u8;128*1024];
        loop {
            let n=f.read(&mut chunk).map_err(|e|e.to_string())?;
            if n==0{break;}
            h.update(&chunk[..n]);
        }
        Ok(h.finalize().to_vec())
    }
    if hash(&temp)?!=hash(cipher)?{return Err("El medio externo no conservó íntegro el archivo cifrado.".into());}
    if destination.exists(){return Err("Otro proceso creó el archivo de destino: no se sobrescribió.".into());}
    fs::rename(&temp,destination).map_err(|e|e.to_string())?;
    Ok(())
}
pub(crate) fn test_archive_at(path:&Path,passphrase:&str)->Result<backup::RecoveryTestResult,String>{
    check_pass(passphrase)?;
    let decoded=read_archive(path,passphrase)?;
    backup::recovery_rehearsal(&decoded.sqlite.with_extension("manifest.json"))
}
pub(crate) fn preview_archive_at(path:&Path,passphrase:&str)->Result<backup::SqlitePreview,String>{
    check_pass(passphrase)?;
    let decoded=read_archive(path,passphrase)?;
    let (data,verified)=backup::read_sqlite_candidate(&decoded.sqlite)?;
    if !verified{return Err("Respaldo no verificado".into());}
    Ok(backup::SqlitePreview{
        data,verified,
        filename:path.file_name().unwrap_or_default().to_string_lossy().into_owned()
    })
}

fn archive_name(result:&backup::BackupResult)->Result<String,String>{
    let filename=Path::new(&result.manifest_path).file_name().and_then(|s|s.to_str())
      .ok_or("Nombre de backup inválido")?;
    let base=filename.strip_suffix(".manifest.json").ok_or("Backup sin manifest")?;
    if !base.starts_with("AutoTrace_")||!base[10..].chars().all(|c|c.is_ascii_alphanumeric()||c=='-'){
        return Err("Nombre de backup incompatible".into());
    }
    Ok(format!("{base}.atbackup.age"))
}
pub(crate) fn seal(result:&backup::BackupResult,dest:&Path,pass:&SecretString)->Result<PathBuf,String>{
    backup::verify_pair_marker(Path::new(&result.manifest_path))?;
    let target=dest.join(archive_name(result)?);
    if target.exists(){return Err("El archivo cifrado ya existe. No se sobrescribió.".into());}
    let staged=dest.join(format!(".autotrace-{}.age.partial",uuid::Uuid::new_v4()));
    let job=(||->Result<(),String>{
        let encrypted=age::Encryptor::with_user_passphrase(pass.clone());
        let file=fs::File::create_new(&staged).map_err(|e|e.to_string())?;
        let mut out=encrypted.wrap_output(BufWriter::new(file)).map_err(|e|e.to_string())?;
        out.write_all(MAGIC).map_err(|e|e.to_string())?;
        for (file_path,limit) in [
            (result.sqlite_path.as_str(),MAX_SQLITE),
            (result.excel_path.as_str(),MAX_EXCEL),
            (result.manifest_path.as_str(),MAX_MANIFEST)
        ] {
            let path=Path::new(file_path);
            let size=fs::metadata(path).map_err(|e|e.to_string())?.len();
            if size==0||size>limit{return Err("Archivo de respaldo excede el tamaño permitido".into());}
            out.write_all(&size.to_le_bytes()).map_err(|e|e.to_string())?;
            let copied=std::io::copy(&mut fs::File::open(path).map_err(|e|e.to_string())?,&mut out).map_err(|e|e.to_string())?;
            if copied!=size{return Err("No se pudo leer el archivo completo".into());}
        }
        let mut file=out.finish().map_err(|e|e.to_string())?;
        file.flush().map_err(|e|e.to_string())?;
        file.get_ref().sync_all().map_err(|e|e.to_string())?;
        drop(file);
        // Never claim success for a ciphertext we cannot recover.
        let recovered=read_archive(&staged,pass.expose_secret_for_restore())?;
        drop(recovered);
        fs::rename(&staged,&target).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if job.is_err(){let _=fs::remove_file(&staged);}
    job?;
    Ok(target)
}
// SecretString's only plaintext exposure is at the decryption boundary.
trait UnsealPassword { fn expose_secret_for_restore(&self)->&str; }
impl UnsealPassword for SecretString {
    fn expose_secret_for_restore(&self)->&str{use age::secrecy::ExposeSecret;self.expose_secret()}
}
// Keep an OS-backed exclusive lock for the full lifetime of decrypted data.
// Another Desktop process must never delete a currently used recovery folder.
struct RestoredArchive { location:PathBuf, sqlite:PathBuf, _lock:Option<fs::File> }
impl Drop for RestoredArchive {
    fn drop(&mut self){
        // Windows refuses to delete open files: release the process lock first.
        drop(self._lock.take());
        let _=fs::remove_dir_all(&self.location);
    }
}
fn unpack<R:Read>(mut reader:R,location:&Path)->Result<PathBuf,String>{
    let mut magic=vec![0u8;MAGIC.len()];
    reader.read_exact(&mut magic).map_err(|e|e.to_string())?;
    if magic!=MAGIC{return Err("Formato del respaldo cifrado no reconocido.".into());}
    // Caller created the exclusive directory and holds its process lock.
    let names=["source.sqlite3","source.xlsx","source.manifest.json"];
    for (name,max) in names.iter().zip([MAX_SQLITE,MAX_EXCEL,MAX_MANIFEST]){
        let mut size_bytes=[0u8;8];
        reader.read_exact(&mut size_bytes).map_err(|e|e.to_string())?;
        let size=u64::from_le_bytes(size_bytes);
        if size<1||size>max{return Err("Contenido del respaldo cifrado excede los límites".into());}
        let mut limited=reader.by_ref().take(size);
        let mut out=fs::File::create_new(location.join(name)).map_err(|e|e.to_string())?;
        let written=std::io::copy(&mut limited,&mut out).map_err(|e|e.to_string())?;
        if written!=size{return Err("El respaldo cifrado está incompleto".into());}
        out.sync_all().map_err(|e|e.to_string())?;
    }
    let mut extra=[0u8;1];
    if reader.read(&mut extra).map_err(|e|e.to_string())?!=0 {
        return Err("El respaldo cifrado contiene bytes adicionales".into());
    }
    let raw=location.join("source.manifest.json");
    let doc:serde_json::Value=serde_json::from_slice(&fs::read(&raw).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let id=doc["snapshotId"].as_str().ok_or("Identificador ausente")?;
    if id.is_empty()||id.len()>120||!id.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'){
        return Err("Identificador cifrado inválido".into());
    }
    let base=format!("AutoTrace_{id}");
    for (old,ext) in [("source.sqlite3","sqlite3"),("source.xlsx","xlsx"),("source.manifest.json","manifest.json")] {
        fs::rename(location.join(old),location.join(format!("{base}.{ext}"))).map_err(|e|e.to_string())?;
    }
    let marker=location.join(format!("{base}.manifest.json"));
    backup::verify_pair_marker(&marker)?;
    Ok(location.join(format!("{base}.sqlite3")))
}
/// A crash can leave decrypted customer data in Temp. Every new unpack
/// holds an OS file lock, so another running process can skip it safely.
/// Legacy directories (created before lock support) retain the 24-hour policy.
fn is_owned_decrypt_dir(path:&Path)->bool{
    let Some(name)=path.file_name().and_then(|s|s.to_str()) else {return false;};
    let Some(id)=name.strip_prefix("autotrace-drive-decrypt-") else {return false;};
    uuid::Uuid::parse_str(id).is_ok()
}
fn cleanup_stale_decrypt_in(base:&Path){
    let Ok(entries)=fs::read_dir(base) else {return};
    for entry in entries.flatten(){
        let path=entry.path();
        if !is_owned_decrypt_dir(&path){continue;}
        let Ok(metadata)=fs::symlink_metadata(&path) else {continue};
        if !metadata.is_dir() || metadata.file_type().is_symlink(){continue;}
        let process_lock=path.join(".autotrace-active.lock");
        if process_lock.exists(){
            // If the lock is held by a second AutoTrace instance, never
            // touch its files. If the OS released the lock after a crash,
            // remove the plaintext on the very next application startup.
            let Ok(file)=fs::OpenOptions::new().read(true).write(true).open(&process_lock) else {continue};
            if file.try_lock().is_err(){continue;}
            drop(file);
            let _=fs::remove_dir_all(&path);
            continue;
        }
        // Backward compatibility for folders left by older builds that
        // cannot establish ownership with an OS-backed lock. Never delete
        // a potentially active legacy recovery based only on its name.
        let Ok(changed)=metadata.modified() else {continue};
        let Ok(age)=std::time::SystemTime::now().duration_since(changed) else {continue};
        if age.as_secs()<24*60*60{continue;}
        let _=fs::remove_dir_all(&path);
    }
}
pub(crate) fn cleanup_stale_decrypt_directories(){
    cleanup_stale_decrypt_in(&std::env::temp_dir());
}
fn read_archive(path:&Path,passphrase:&str)->Result<RestoredArchive,String>{
    let meta=fs::metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.len()>600_000_000||meta.len()<100 {
        return Err("Archivo cifrado vacío o demasiado grande".into());
    }
    let decryptor=age::Decryptor::new(BufReader::new(fs::File::open(path).map_err(|e|e.to_string())?))
      .map_err(|_|"No se reconoce el archivo de respaldo cifrado")?;
    let identity=age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));
    let reader=decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
      .map_err(|_|"Clave incorrecta o archivo cifrado alterado")?;
    let location=std::env::temp_dir().join(format!("autotrace-drive-decrypt-{}",uuid::Uuid::new_v4()));
    fs::create_dir(&location).map_err(|_|"No se pudo preparar la recuperación temporal")?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    let cleanup=Cleanup(location.clone());
    let lock=fs::OpenOptions::new().read(true).write(true).create_new(true)
        .open(location.join(".autotrace-active.lock"))
        .map_err(|_|"No se pudo asegurar la recuperación temporal")?;
    lock.lock().map_err(|_|"No se pudo bloquear la carpeta temporal de recuperación")?;
    let sqlite=unpack(reader,&location).map_err(|_|"Clave incorrecta, datos incompletos o respaldo cifrado dañado")?;
    std::mem::forget(cleanup);
    Ok(RestoredArchive{location,sqlite,_lock:Some(lock)})
}
pub(crate) fn replicate_if_unlocked(app:&tauri::AppHandle,result:&backup::BackupResult)->Result<Option<String>,String>{
    let Some(dir)=configured_directory(app)? else {return Ok(None)};
    if !dir.is_dir(){return Err("No se encuentra la carpeta de Google Drive configurada".into());}
    reject_plaintext_in_sync_folder(&dir)?;
    let pass=phrase_for_backup()?.ok_or("La copia de Google Drive está bloqueada: desbloqueala en Ajustes.")?;
    seal(result,&dir,&pass).map(|p|Some(p.display().to_string()))
}
/// Verify a downloaded encrypted archive against both fresh SQLite and Excel stores,
/// without modifying the active workshop database.
#[tauri::command]
pub(crate) fn test_drive_recovery(passphrase:String)->Result<Option<backup::RecoveryTestResult>,String>{
    check_pass(&passphrase)?;
    let Some(path)=rfd::FileDialog::new()
      .set_title("Elegí la copia cifrada descargada de Google Drive que querés comprobar")
      .add_filter("Respaldo AutoTrace cifrado",&["age"]).pick_file() else{return Ok(None)};
    let archive=read_archive(&path,&passphrase)?;
    let manifest=archive.sqlite.with_extension("manifest.json");
    backup::recovery_rehearsal(&manifest).map(Some)
}
#[tauri::command]
pub(crate) fn choose_drive_restore(passphrase:String)->Result<Option<backup::SqlitePreview>,String>{
    check_pass(&passphrase)?;
    let Some(path)=rfd::FileDialog::new()
      .set_title("Elegí un backup AutoTrace cifrado descargado de Google Drive")
      .add_filter("Respaldo AutoTrace cifrado",&["age"]).pick_file() else{return Ok(None)};
    let recovered=read_archive(&path,&passphrase)?;
    let (data,verified)=backup::read_sqlite_candidate(&recovered.sqlite)?;
    if !verified{return Err("Respaldo sin comprobación de integridad".into());}
    Ok(Some(backup::SqlitePreview{
        data,verified:true,
        filename:path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;
    use rusqlite::Connection;
    #[test]
    fn plaintext_backup_names_are_blocked_in_a_sync_directory(){
        assert!(is_plain_workshop_backup_name("AutoTrace_2026-01.sqlite3"));
        assert!(is_plain_workshop_backup_name("AutoTrace_2026-01.xlsx"));
        assert!(is_plain_workshop_backup_name("AutoTrace_2026-01.manifest.json"));
        assert!(!is_plain_workshop_backup_name("AutoTrace_2026-01.atbackup.age"));
        assert!(!is_plain_workshop_backup_name("Comprobante_cliente.xlsx"));
    }
    #[test]
    fn temp_cleanup_only_targets_our_strict_uuid_directory_names(){
        let owned=std::env::temp_dir().join(format!("autotrace-drive-decrypt-{}",uuid::Uuid::new_v4()));
        assert!(is_owned_decrypt_dir(&owned));
        for forbidden in ["autotrace-drive-decrypt-", "autotrace-drive-decrypt-garbage",
            "autotrace-drive-decrypt-bad/path", "autotrace-test-drive-decrypt-customer",
            "AutoTrace_secret_customer","some-other-temp-directory"]{
            assert!(!is_owned_decrypt_dir(&std::env::temp_dir().join(forbidden)));
        }
    }
    #[test]
    fn crash_cleanup_removes_only_unlocked_own_plaintext_and_preserves_active_recovery(){
        let root=std::env::temp_dir().join(format!("autotrace-cleanup-isolated-{}",uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let orphan=root.join(format!("autotrace-drive-decrypt-{}",uuid::Uuid::new_v4()));
        let active=root.join(format!("autotrace-drive-decrypt-{}",uuid::Uuid::new_v4()));
        let unrelated=root.join("customer-files-do-not-touch");
        let legacy=root.join(format!("autotrace-drive-decrypt-{}",uuid::Uuid::new_v4()));
        for d in [&orphan,&active,&unrelated,&legacy]{fs::create_dir(d).unwrap();}
        fs::write(orphan.join("plaintext.sqlite3"),b"synthetic-orphan").unwrap();
        fs::write(active.join("plaintext.sqlite3"),b"synthetic-active").unwrap();
        fs::write(unrelated.join("important.txt"),b"not AutoTrace").unwrap();
        fs::write(legacy.join("source.sqlite3"),b"legacy-decrypt").unwrap();
        fs::write(orphan.join(".autotrace-active.lock"),b"").unwrap();
        let lock=fs::OpenOptions::new().read(true).write(true).create_new(true)
            .open(active.join(".autotrace-active.lock")).unwrap();
        lock.lock().unwrap();
        cleanup_stale_decrypt_in(&root);
        assert!(!orphan.exists(),"orphaned sensitive data must be cleared at restart");
        assert!(active.join("plaintext.sqlite3").exists(),"a live restore in another instance cannot be deleted");
        assert!(legacy.exists(),"legacy folders without locks need the conservative grace period");
        assert!(unrelated.join("important.txt").exists(),"foreign directories must survive cleanup");
        drop(lock);
        cleanup_stale_decrypt_in(&root);
        assert!(!active.exists(),"once the OS releases the lock, a leftover archive must be purged");
        assert!(legacy.exists());
        assert!(unrelated.exists());
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn generated_recovery_key_is_high_entropy_and_portable(){
        let a=generate_recovery_phrase();
        let b=generate_recovery_phrase();
        assert!(a.starts_with("AT-"));
        assert_eq!(a.len(),68);
        assert!(a.bytes().all(|v|v.is_ascii_alphanumeric()||v==b'-'));
        assert_ne!(a,b);
        check_pass(&a).unwrap();
    }
    #[test]
    fn portable_copy_is_atomic_no_overwrite_and_keeps_cipher_source(){
        let folder=std::env::temp_dir().join(format!("autotrace-portable-test-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let source=folder.join("sealed.age");
        let destination=folder.join("Backup.atbackup.age");
        let payload=vec![0x5a;128*1024+133];
        fs::write(&source,&payload).unwrap();
        copy_verified_cipher(&source,&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(),payload);
        assert_eq!(fs::read(&source).unwrap(),payload);
        assert!(copy_verified_cipher(&source,&destination).is_err(),"no overwrite of prior backup");
        assert_eq!(fs::read(&destination).unwrap(),payload);
        let partials=fs::read_dir(&folder).unwrap()
            .filter_map(Result::ok).filter(|f|f.file_name().to_string_lossy().ends_with(".partial")).count();
        assert_eq!(partials,0,"no partial files after successful or rejected export");
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn plain_and_encrypted_backup_folders_cannot_overlap(){
        assert!(paths_overlap(Path::new("G:/Mi unidad/AutoTrace"),Path::new("G:/Mi unidad/AutoTrace")));
        assert!(paths_overlap(Path::new("G:/Mi unidad"),Path::new("G:/Mi unidad/AutoTrace")));
        assert!(paths_overlap(Path::new("G:/Mi unidad/AutoTrace"),Path::new("G:/Mi unidad/AutoTrace/plain")));
        assert!(!paths_overlap(Path::new("D:/USB/backups"),Path::new("G:/Mi unidad/AutoTrace")));
    }
    #[test]
    fn cloud_archive_is_encrypted_recovers_both_formats_and_rejects_wrong_key(){
        let temp=std::env::temp_dir().join(format!("autotrace-drive-test-{}",uuid::Uuid::new_v4()));
        let primary=temp.join("primary");let drive=temp.join("synced");
        fs::create_dir_all(&primary).unwrap();fs::create_dir_all(&drive).unwrap();
        let mut conn=Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        let doc:store::AppData=serde_json::from_value(serde_json::json!({
          "clients":[{"id":"c1","name":"Cliente protegido","phone":"0112345000","email":"","notes":"","createdAt":"2026-10-20"}],
          "vehicles":[{"id":"v1","clientId":"c1","plate":"AF123BD","make":"Ford","model":"Ka","year":2019,"mileage":1.0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[{"id":"o1","vehicleId":"v1","title":"Diagnóstico","category":"Electrónica","status":"done","openedAt":"2026-10-22","completedAt":"","mileage":1.0,"description":"","partsCost":150.0,"laborCost":200.0,"charged":1000.0,"paid":500.0,"notes":"","createdAt":"2026-10-22","payments":[{"id":"pay1","date":"2026-10-22","amount":200.0,"method":"transfer","accountId":"account1","reference":"OPERACION_SECRETA"}]}],
          "settings":{"garageName":"Taller","currency":"ARS","demo":false,"accounts":[{"id":"account1","name":"Banco protegido","kind":"bank","alias":"ALIAS_SECRETO"}]}
        })).unwrap();
        store::write(&mut conn,&doc,0).unwrap();
        let pair=backup::backup_pair(&conn,&primary,"fixture").unwrap();
        let password=SecretString::from("una clave de prueba larga y privada".to_owned());
        let cloud=seal(&pair,&drive,&password).unwrap();
        let raw=fs::read(&cloud).unwrap();
        assert!(!raw.windows(b"Cliente protegido".len()).any(|chunk|chunk==b"Cliente protegido"));
        assert!(!raw.windows(b"SQLite format 3".len()).any(|chunk|chunk==b"SQLite format 3"));
        assert!(!raw.windows(b"OPERACION_SECRETA".len()).any(|chunk|chunk==b"OPERACION_SECRETA"));
        assert!(!raw.windows(b"ALIAS_SECRETO".len()).any(|chunk|chunk==b"ALIAS_SECRETO"));
        let recovered=read_archive(&cloud,password.expose_secret_for_restore()).unwrap();
        let (restored,verified)=backup::read_sqlite_candidate(&recovered.sqlite).unwrap();
        assert!(verified);
        assert_eq!(excel_canonical(&restored),excel_canonical(&doc));
        // Independent recovery into TWO clean SQLite databases (one from SQLite,
        // one from Excel), with ledger and receiver accounts preserved.
        let rehearsal=backup::recovery_rehearsal(&recovered.sqlite.with_extension("manifest.json")).unwrap();
        assert!(rehearsal.sqlite_and_excel_match);
        assert_eq!(rehearsal.clients,1);
        assert_eq!(rehearsal.vehicles,1);
        assert_eq!(rehearsal.orders,1);
        assert!(verify_pending_archives(&[cloud.clone()],password.expose_secret_for_restore()).is_ok());
        // A corrupted/mismatched SECOND item must never be ignored just because
        // the FIRST encrypted backup validates with the supplied recovery key.
        let other_dir=temp.join("different-key");
        fs::create_dir_all(&other_dir).unwrap();
        let wrong_key=SecretString::from("another very long synthetic recovery passphrase".to_owned());
        let mismatched=seal(&pair,&other_dir,&wrong_key).unwrap();
        // A folder can have the *first and last* backups under the correct
        // phrase while a middle historical snapshot was encrypted differently.
        // Checking only both ends used to accept this unsafe folder.
        let mixed_history=[cloud.clone(),mismatched.clone(),cloud.clone()];
        assert!(verify_synced_archive_keys(&mixed_history,password.expose_secret_for_restore()).is_err());
        assert!(verify_synced_archive_keys(&[cloud.clone(),cloud.clone()],password.expose_secret_for_restore()).is_ok());
        assert!(verify_pending_archives(&[cloud.clone(),mismatched],password.expose_secret_for_restore()).is_err());
        assert!(verify_pending_archives(&vec![cloud.clone();51],password.expose_secret_for_restore()).is_err());
        assert!(read_archive(&cloud,"wrong passphrase more than sixteen").is_err());
        assert!(seal(&pair,&drive,&password).is_err(),"must never overwrite earlier cloud backups");
        let mut damaged=raw.clone();let idx=damaged.len()-33;damaged[idx]^=0x6a;
        let tampered=drive.join("tampered.age");fs::write(&tampered,damaged).unwrap();
        assert!(read_archive(&tampered,password.expose_secret_for_restore()).is_err());
        // G4: interrupted copies must never pass the encrypted-archive checks,
        // even when most of the age stream was written successfully.
        for cut in [0,raw.len()/2,raw.len()-1] {
            let truncated=drive.join(format!("truncated-{cut}.atbackup.age"));
            fs::write(&truncated,&raw[..cut]).unwrap();
            assert!(test_archive_at(&truncated,password.expose_secret_for_restore()).is_err(),
                "a {cut}-byte truncated archive must fail the recovery rehearsal");
            assert!(preview_archive_at(&truncated,password.expose_secret_for_restore()).is_err(),
                "a truncated archive must not expose a restorable preview");
        }
        drop(recovered);
        fs::remove_dir_all(temp).unwrap();
    }
    fn excel_canonical(data:&store::AppData)->Vec<u8>{crate::excel::canonical_bytes(data).unwrap()}
}