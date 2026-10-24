//! Strict Google Drive v3 encrypted-file transport.
//! All HTTP destinations are Google HTTPS endpoints; no public sharing endpoints.
use md5;
use reqwest::{blocking::{Body,Client},header::{CONTENT_LENGTH,CONTENT_RANGE,CONTENT_TYPE,LOCATION,RANGE}};
use serde::{Serialize,Deserialize};
use std::{fs,io::{Read,Seek,SeekFrom},path::Path,thread,time::Duration};
use url::Url;
const API:&str="https://www.googleapis.com/drive/v3/files";
const UPLOAD:&str="https://www.googleapis.com/upload/drive/v3/files";
const MAX:u64=600_000_000;
const CHUNK:u64=8*1024*1024;
fn file_md5(path:&Path)->Result<String,String>{
    let mut file=fs::File::open(path).map_err(|e|e.to_string())?;
    let mut digest=md5::Context::new();
    let mut buf=[0u8;128*1024];
    loop{
        let n=file.read(&mut buf).map_err(|e|e.to_string())?;
        if n==0{break;}
        digest.consume(&buf[..n]);
    }
    Ok(format!("{:x}",digest.compute()))
}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct CloudFile {
    pub id:String,
    pub name:String,
    pub size:Option<String>,
    pub md5_checksum:Option<String>,
    pub created_time:Option<String>,
}
fn http()->Result<Client,String>{
    Client::builder().timeout(Duration::from_secs(90)).build().map_err(|_|"No se pudo iniciar HTTPS".into())
}
fn identifier(id:&str)->Result<&str,String>{
    if id.len()<8||id.len()>160||!id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_'){
        return Err("ID remoto inválido".into());
    }
    Ok(id)
}
fn json_get(client:&Client,token:&str,url:&str)->Result<serde_json::Value,String>{
    let r=client.get(url).bearer_auth(token).send().map_err(|_|"No hay conexión con Google Drive")?;
    if !r.status().is_success(){return Err(format!("Google Drive devolvió HTTP {}. Revisá la cuenta, la cuota o la conexión.",r.status().as_u16()));}
    r.json().map_err(|_|"La respuesta de Drive no es válida".into())
}
fn json_post(client:&Client,token:&str,url:&str,data:&serde_json::Value)->Result<serde_json::Value,String>{
    let r=client.post(url).bearer_auth(token).json(data).send().map_err(|_|"No se pudo contactar con Google Drive")?;
    if !r.status().is_success(){return Err(format!("Google Drive rechazó la creación: HTTP {}",r.status().as_u16()));}
    r.json().map_err(|_|"La respuesta de Drive no es válida".into())
}
fn list_query(client:&Client,token:&str,q:&str,fields:&str)->Result<Vec<serde_json::Value>,String>{
    let mut all=Vec::new();
    let mut cursor:Option<String>=None;
    for _page in 0..20 {
        let mut url=Url::parse(API).map_err(|e|e.to_string())?;
        url.query_pairs_mut().append_pair("q",q).append_pair("fields",fields)
            .append_pair("pageSize","1000");
        if let Some(ref token)=cursor{url.query_pairs_mut().append_pair("pageToken",token);}
        let doc=json_get(client,token,url.as_str())?;
        let page=doc["files"].as_array().ok_or("Google no devolvió una lista de archivos")?;
        all.extend(page.iter().cloned());
        cursor=doc["nextPageToken"].as_str().filter(|v|!v.is_empty()).map(str::to_owned);
        if cursor.is_none(){return Ok(all);}
    }
    Err("Se alcanzó el límite de paginación de Drive. Se conservarán todas las copias; revisá la política de retención remota.".into())
}
fn folder(client:&Client,token:&str)->Result<String,String>{
    let q="trashed = false and mimeType = 'application/vnd.google-apps.folder' and appProperties has { key='autotraceKind' and value='backups' }";
    let list=list_query(client,token,q,"nextPageToken,files(id,name)")?;
    if let Some(f)=list.first(){return Ok(identifier(f["id"].as_str().ok_or("Carpeta sin ID")?)?.to_owned());}
    let create=serde_json::json!({
        "name":"AutoTrace Backups","mimeType":"application/vnd.google-apps.folder",
        "appProperties":{"autotraceKind":"backups"}
    });
    let result=json_post(client,token,API,&create)?;
    Ok(identifier(result["id"].as_str().ok_or("Falta ID de carpeta creada")?)?.to_owned())
}
fn listed(client:&Client,token:&str)->Result<Vec<CloudFile>,String>{
    let q="trashed = false and appProperties has { key='autotraceBackup' and value='age-v1' }";
    let raw=list_query(client,token,q,"nextPageToken,files(id,name,size,md5Checksum,createdTime)")?;
    let mut files=Vec::new();
    for item in raw{
        let f:CloudFile=serde_json::from_value(item).map_err(|_|"Archivo remoto no válido")?;
        if f.name.starts_with("AutoTrace_")&&f.name.ends_with(".atbackup.age")&&identifier(&f.id).is_ok(){files.push(f);}
    }
    files.sort_by(|a,b|b.created_time.cmp(&a.created_time));
    Ok(files)
}
pub(crate) fn list(token:&str)->Result<Vec<CloudFile>,String>{listed(&http()?,token)}
/// A remote name alone is not an identity. Ambiguous names are not a verified
/// backup and must be reconciled manually rather than silently accepted.
fn already_uploaded(files:&[CloudFile],name:&str,size:u64,checksum:&str)->Result<Option<String>,String>{
    let matches:Vec<_>=files.iter().filter(|f|f.name==name).collect();
    if matches.len()>1{
        return Err("Hay varias copias de igual nombre en Drive. No se puede verificar unívocamente; revisalas antes de reintentar.".into());
    }
    let Some(existing)=matches.first() else{return Ok(None)};
    if existing.size.as_deref()==Some(size.to_string().as_str())&&
       existing.md5_checksum.as_deref()==Some(checksum){
        return Ok(Some(existing.id.clone()));
    }
    Err("Ya existe un backup remoto de igual nombre y distinto contenido. No se sobrescribe.".into())
}
fn next_offset(range:Option<&str>,size:u64)->Result<u64,String>{
    let Some(r)=range else {return Ok(0)};
    let v=r.strip_prefix("bytes=0-").ok_or("Google devolvió un rango inesperado")?
        .parse::<u64>().map_err(|_|"Rango de subida inválido")?;
    if v>=size{return Err("Rango confirmado excede el archivo".into());}
    Ok(v+1)
}
pub(crate) fn upload(token:&str,path:&Path)->Result<String,String>{
    let client=http()?;
    let name=path.file_name().and_then(|n|n.to_str()).ok_or("Archivo sin nombre")?;
    if !name.starts_with("AutoTrace_")||!name.ends_with(".atbackup.age"){return Err("Solo se admiten archivos AutoTrace cifrados".into());}
    let size=fs::metadata(path).map_err(|e|e.to_string())?.len();
    if size==0||size>MAX{return Err("Archivo cifrado fuera del tamaño permitido".into());}
    let checksum=file_md5(path)?;
    // Retry idempotently, including a completed request whose local acknowledgement was lost.
    if let Some(id)=already_uploaded(&listed(&client,token)?,name,size,&checksum)?{return Ok(id);}
    let parent=folder(&client,token)?;
    let info=serde_json::json!({
        "name":name,"mimeType":"application/octet-stream","parents":[parent],
        "appProperties":{"autotraceBackup":"age-v1"}
    });
    let init=client.post(format!("{UPLOAD}?uploadType=resumable&fields=id,name,size,md5Checksum"))
        .bearer_auth(token).header("X-Upload-Content-Type","application/octet-stream")
        .header("X-Upload-Content-Length",size.to_string()).json(&info).send()
        .map_err(|_|"No se pudo iniciar la subida reanudable a Google Drive")?;
    if !init.status().is_success(){return Err(format!("Google rechazó la subida: HTTP {}",init.status()));}
    let session=init.headers().get(LOCATION).and_then(|x|x.to_str().ok()).ok_or("Falta sesión de subida de Google")?;
    let url=Url::parse(session).map_err(|_|"Sesión de subida inválida")?;
    if url.scheme()!="https"||url.host_str()!=Some("www.googleapis.com")||
       !url.path().starts_with("/upload/drive/v3/files"){
        return Err("Google entregó una URL de subida no confiable".into());
    }
    let id=upload_chunks(&client,token,&url,path,size,CHUNK)?;

    let mut url=Url::parse(&format!("{API}/{id}")).map_err(|e|e.to_string())?;
    url.query_pairs_mut().append_pair("fields","id,size,md5Checksum,appProperties");
    let remote=json_get(&client,token,url.as_str())?;
    if remote["size"].as_str()!=Some(size.to_string().as_str())||
       remote["md5Checksum"].as_str()!=Some(checksum.as_str())||
       remote["appProperties"]["autotraceBackup"]!="age-v1"{
        return Err("La copia está subida pero falló la verificación de tamaño y checksum.".into());
    }
    Ok(id)
}
/// Transfer a resumable session in bounded chunks. Keeping this separate
/// allows deterministic loopback tests without Google credentials.
fn upload_chunks(client:&Client,token:&str,url:&Url,path:&Path,size:u64,chunk_size:u64)->Result<String,String>{
    if chunk_size==0{return Err("Tamaño de fragmento inválido".into());}
    let mut offset=0u64;
    let mut retries=0usize;
    let mut id=None;
    while offset<size {
        let end=(offset+chunk_size).min(size)-1;
        let mut f=fs::File::open(path).map_err(|e|e.to_string())?;
        f.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;
        let len=end-offset+1;
        let response=client.put(url.clone()).bearer_auth(token)
            .header(CONTENT_TYPE,"application/octet-stream")
            .header(CONTENT_RANGE,format!("bytes {offset}-{end}/{size}"))
            .header(CONTENT_LENGTH,len.to_string())
            .body(Body::sized(f.take(len),len)).send();
        match response {
            Ok(r) if r.status().is_success()=>{
                let doc:serde_json::Value=r.json().map_err(|_|"Respuesta de subida sin JSON")?;
                id=Some(identifier(doc["id"].as_str().ok_or("Falta ID de subida")?)?.to_owned());break;
            },
            Ok(r) if r.status().as_u16()==308=>{
                let new=next_offset(r.headers().get(RANGE).and_then(|x|x.to_str().ok()),size)?;
                if new<=offset {
                    // Google may return 308 without Range before persisting a chunk.
                    // Do not claim progress; retry after a bounded status probe.
                    retries+=1;
                }else{
                    offset=new;retries=0;
                }
            },
            Ok(r) if r.status().is_server_error()||r.status().as_u16()==429=>{retries+=1;},
            Err(_)=>{retries+=1;},
            Ok(r)=>return Err(format!("Drive rechazó el archivo (HTTP {})",r.status())),
        }
        if retries>4{return Err("Subida interrumpida. La copia cifrada sigue pendiente localmente.".into());}
        if retries>0{
            thread::sleep(Duration::from_secs(1u64<<retries.min(3)));
            let probe=client.put(url.clone()).bearer_auth(token)
                .header(CONTENT_RANGE,format!("bytes */{size}"))
                .header(CONTENT_LENGTH,"0").send().map_err(|_|"No se pudo reanudar la carga")?;
            if probe.status().as_u16()==308{
                offset=next_offset(probe.headers().get(RANGE).and_then(|x|x.to_str().ok()),size)?;
            }else if probe.status().is_success(){
                let doc:serde_json::Value=probe.json().map_err(|_|"Confirmación remota inválida")?;
                id=Some(identifier(doc["id"].as_str().ok_or("Falta ID remoto")?)?.to_owned());break;
            }else{return Err("No se pudo reanudar la subida. La copia sigue pendiente localmente.".into());}
        }
    }
    id.ok_or("No se completó la subida de Drive".into())
}
pub(crate) fn download(token:&str,id:&str,destination:&Path)->Result<(),String>{
    let client=http()?;
    let selected=listed(&client,token)?.into_iter().find(|x|x.id==id)
        .ok_or("El archivo no pertenece a las copias de AutoTrace en esta cuenta")?;
    identifier(id)?;
    let size=selected.size.as_deref().ok_or("Drive no devuelve el tamaño")?
        .parse::<u64>().map_err(|_|"Tamaño remoto inválido")?;
    if size==0||size>MAX{return Err("Copia remota fuera de tamaño permitido".into());}
    let response=client.get(format!("{API}/{id}?alt=media")).bearer_auth(token).send()
        .map_err(|_|"Error descargando desde Google Drive")?;
    if !response.status().is_success(){return Err(format!("Drive rechazó descarga (HTTP {})",response.status()));}
    let expected_md5=selected.md5_checksum.as_deref().ok_or("Drive no devolvió checksum del archivo")?;
    receive_verified(response,size,expected_md5,destination)
}
/// Write only to an exclusive staging file; publish the downloaded ciphertext
/// after comparing the exact remote size and checksum. Any network or integrity
/// failure removes the temporary file and leaves the destination unchanged.
fn receive_verified<R:Read>(stream:R,size:u64,expected_md5:&str,destination:&Path)->Result<(),String>{
    if size==0||size>MAX{return Err("Tamaño de descarga fuera de los límites".into());}
    if expected_md5.len()!=32||!expected_md5.bytes().all(|c|c.is_ascii_hexdigit()){
        return Err("Checksum remoto ausente o inválido".into());
    }
    if destination.exists(){return Err("Ya existe un archivo en destino: no se sobrescribió".into());}
    let parent=destination.parent().ok_or("Destino de descarga inválido")?;
    let stage=parent.join(format!(".autotrace-download-{}.partial",uuid::Uuid::new_v4()));
    struct RemoveStage(std::path::PathBuf);
    impl Drop for RemoveStage{
        fn drop(&mut self){let _=fs::remove_file(&self.0);}
    }
    let guard=RemoveStage(stage.clone());
    let mut file=fs::OpenOptions::new().write(true).create_new(true)
        .open(&stage).map_err(|e|e.to_string())?;
    let copied=std::io::copy(&mut stream.take(size+1),&mut file).map_err(|_|"Descarga interrumpida")?;
    if copied!=size{return Err("Copia descargada incompleta o fuera de tamaño".into());}
    file.sync_all().map_err(|e|e.to_string())?;
    drop(file);
    if file_md5(&stage)?.to_ascii_lowercase()!=expected_md5.to_ascii_lowercase(){
        return Err("Google devolvió una copia con checksum distinto".into());
    }
    if destination.exists(){return Err("Otro proceso creó el archivo destino: no se sobrescribió".into());}
    fs::rename(&stage,destination).map_err(|e|e.to_string())?;
    drop(guard);
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn mock_google_http_errors_are_safe_and_non_retryable_without_bytes(){
        use std::io::{Read,Write};
        use std::net::TcpListener;
        for status in [401,403,429,500]{
            let listener=TcpListener::bind("127.0.0.1:0").unwrap();
            let addr=listener.local_addr().unwrap();
            let server=std::thread::spawn(move ||{
                let (mut socket,_)=listener.accept().unwrap();
                socket.set_read_timeout(Some(std::time::Duration::from_secs(4))).unwrap();
                let mut request=[0u8;8192];
                let _=socket.read(&mut request).unwrap();
                let response=format!("HTTP/1.1 {status} MOCK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}");
                socket.write_all(response.as_bytes()).unwrap();
            });
            let endpoint=format!("http://{addr}/drive/v3/files");
            let result=json_get(&http().unwrap(),"synthetic-test-token",&endpoint);
            assert!(result.is_err(),"status {status} cannot be treated as successful Drive verification");
            assert!(result.unwrap_err().contains(&status.to_string()));
            server.join().unwrap();
        }
    }
    #[test] fn bad_mock_json_is_not_accepted_as_remote_confirmation(){
        use std::io::{Read,Write};
        use std::net::TcpListener;
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let addr=listener.local_addr().unwrap();
        let server=std::thread::spawn(move ||{
            let (mut s,_)=listener.accept().unwrap();
            s.set_read_timeout(Some(std::time::Duration::from_secs(4))).unwrap();
            let mut req=[0u8;8192];let _=s.read(&mut req).unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nNOTJS").unwrap();
        });
        let result=json_get(&http().unwrap(),"synthetic-test-token",&format!("http://{addr}/drive/v3/files"));
        assert!(result.is_err());
        server.join().unwrap();
    }
    /// Simulate a provider that acknowledges chunk 0 with 308 and finishes
    /// the next chunk. No Google credentials or network outside loopback.
    #[test] fn resumable_transfer_obeys_server_confirmed_ranges(){
        use std::io::{Read,Write};
        use std::net::TcpListener;
        let dir=std::env::temp_dir().join(format!("autotrace-drive-chunks-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file=dir.join("encrypted.age");
        fs::write(&file,b"ABCDEFGH").unwrap();
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let addr=listener.local_addr().unwrap();
        let server=std::thread::spawn(move ||{
            let mut ranges=Vec::new();
            for round in 0..2 {
                let (mut socket,_)=listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut collected=Vec::new();
                let mut buf=[0u8;2048];
                loop{
                    let n=socket.read(&mut buf).unwrap();
                    assert!(n>0,"upload request ended before content");
                    collected.extend_from_slice(&buf[..n]);
                    let Some(head_end)=collected.windows(4).position(|c|c==b"\r\n\r\n") else {continue};
                    let headers=String::from_utf8_lossy(&collected[..head_end]).to_ascii_lowercase();
                    let size=headers.lines().find_map(|line|line.strip_prefix("content-length:"))
                        .unwrap().trim().parse::<usize>().unwrap();
                    if collected.len()<head_end+4+size{continue;}
                    let range=headers.lines().find(|line|line.starts_with("content-range:")).unwrap().to_owned();
                    ranges.push(range);
                    let body=&collected[head_end+4..head_end+4+size];
                    assert_eq!(body,if round==0{b"ABCD"}else{b"EFGH"});
                    break;
                }
                if round==0{
                    socket.write_all(b"HTTP/1.1 308 Resume Incomplete\r\nRange: bytes=0-3\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                }else{
                    let payload=r#"{"id":"mock-verified-123"}"#;
                    write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",payload.len(),payload).unwrap();
                }
            }
            ranges
        });
        let client=http().unwrap();
        let endpoint=Url::parse(&format!("http://{addr}/upload-session")).unwrap();
        let result=upload_chunks(&client,"synthetic-token",&endpoint,&file,8,4).unwrap();
        assert_eq!(result,"mock-verified-123");
        assert_eq!(server.join().unwrap(),vec!["content-range: bytes 0-3/8","content-range: bytes 4-7/8"]);
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test] fn malformed_confirmed_range_cannot_publish_remote_success(){
        use std::io::{Read,Write};
        use std::net::TcpListener;
        let dir=std::env::temp_dir().join(format!("autotrace-drive-invalid-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path=dir.join("sealed.age");
        fs::write(&path,b"12345678").unwrap();
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let addr=listener.local_addr().unwrap();
        let server=std::thread::spawn(move ||{
            let (mut socket,_)=listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request=[0u8;4096];
            let _=socket.read(&mut request).unwrap();
            socket.write_all(b"HTTP/1.1 308 Resume Incomplete\r\nRange: bytes=0-99\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let client=http().unwrap();
        let endpoint=Url::parse(&format!("http://{addr}/upload-session")).unwrap();
        let result=upload_chunks(&client,"synthetic-token",&endpoint,&path,8,4);
        assert!(result.is_err(),"an impossible Range must never complete the upload");
        server.join().unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test] fn interrupted_and_corrupt_downloads_never_publish_files(){
        let dir=std::env::temp_dir().join(format!("autotrace-download-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let destination=dir.join("download.atbackup.age");
        let data=b"encrypted synthetic content only";
        let md5=format!("{:x}",md5::compute(data));
        for (bytes,size,checksum) in [
            (&data[..data.len()-1],data.len() as u64,md5.as_str()),
            (&data[..],(data.len()-1) as u64,md5.as_str()),
            (&data[..],data.len() as u64,"00000000000000000000000000000000")
        ] {
            assert!(receive_verified(bytes,size,checksum,&destination).is_err());
            assert!(!destination.exists(),"a failed transfer must not publish ciphertext");
            assert_eq!(fs::read_dir(&dir).unwrap().count(),0,"failed downloads must clean staging");
        }
        receive_verified(&data[..],data.len() as u64,&md5,&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(),data);
        assert!(receive_verified(b"overwrite",9,&md5,&destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(),data);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn only_one_matching_remote_backup_may_be_reused(){
        let good=CloudFile{
            id:"validRemoteId_123".into(),name:"AutoTrace_123.atbackup.age".into(),
            size:Some("123".into()),md5_checksum:Some("a".repeat(32)),created_time:None
        };
        assert_eq!(already_uploaded(&[],&good.name,123,&"a".repeat(32)).unwrap(),None);
        assert_eq!(already_uploaded(&[good.clone()],&good.name,123,&"a".repeat(32)).unwrap(),Some(good.id.clone()));
        assert!(already_uploaded(&[good.clone()],&good.name,124,&"a".repeat(32)).is_err());
        assert!(already_uploaded(&[good.clone(),good.clone()],&good.name,123,&"a".repeat(32)).is_err());
    }
    #[test] fn ranges_and_ids_are_checked(){
        assert_eq!(next_offset(None,20).unwrap(),0);
        assert_eq!(next_offset(Some("bytes=0-7"),20).unwrap(),8);
        assert!(next_offset(Some("bytes=0-20"),20).is_err());
        assert!(identifier("../../etc/passwd").is_err());
        assert!(identifier("validFileId_22").is_ok());
    }
}