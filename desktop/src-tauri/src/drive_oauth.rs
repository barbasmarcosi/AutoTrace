//! Public-client Google OAuth for a Windows Desktop application: PKCE S256 + loopback.
//! Refresh token is stored in Windows Credential Manager through keyring.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD,Engine as _};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::{io::{Read,Write},net::TcpListener,thread,time::{Duration,Instant}};
use url::Url;
const SERVICE:&str="AutoTrace-GoogleDrive";
const USER:&str="refresh-token";
pub(crate) const SCOPE:&str="https://www.googleapis.com/auth/drive.file";
fn vault()->Result<keyring::Entry,String>{
    keyring::Entry::new(SERVICE,USER).map_err(|_|"No se pudo acceder a credenciales seguras de Windows".into())
}
pub(crate) fn authorized()->bool{vault().is_ok_and(|v|v.get_password().is_ok())}
/// Do not report a successful disconnect while Windows still stores a
/// refresh token. Revocation at Google remains a separate user action.
fn finish_forget(result:Result<(),keyring::Error>)->Result<(),String>{
    match result{
        Ok(())|Err(keyring::Error::NoEntry)=>Ok(()),
        Err(_)=>Err("Windows no pudo eliminar la autorización local de Google. No se confirmó la desconexión.".into()),
    }
}
pub(crate) fn forget()->Result<(),String>{
    finish_forget(vault()?.delete_credential())
}
pub(crate) fn client_id_valid(s:&str)->bool{
    s.len()>35&&s.len()<300&&s.ends_with(".apps.googleusercontent.com")
        &&s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'.'||b==b'-')
}
fn http()->Result<Client,String>{
    Client::builder().timeout(Duration::from_secs(80)).build().map_err(|_|"Error inicializando HTTPS".into())
}
#[derive(Deserialize)]
struct Tokens {access_token:Option<String>,refresh_token:Option<String>}
/// Dependency-injected HTTP boundary allows repeatable local-provider tests;
/// the production endpoint remains hard-coded to the official Google origin.
fn exchange_with(client:&Client,endpoint:&str,fields:&[(&str,&str)])->Result<Tokens,String>{
    let resp=client.post(endpoint).form(fields).send()
        .map_err(|_|"No se pudo comunicar con OAuth de Google")?;
    if !resp.status().is_success(){return Err(format!("Google OAuth rechazó el acceso (HTTP {}). Revisá el Client ID o volvé a conectar.",resp.status().as_u16()));}
    resp.json().map_err(|_|"La respuesta de Google OAuth no es válida".into())
}
fn exchange(fields:&[(&str,&str)])->Result<Tokens,String>{
    exchange_with(&http()?,"https://oauth2.googleapis.com/token",fields)
}
/// The rotated refresh token must be persisted before the access token is
/// returned. A vault failure cannot be represented as an authenticated user.
fn access_from_refresh_reply(
    reply:Tokens,previous:&str,mut persist:impl FnMut(&str)->Result<(),String>
)->Result<String,String>{
    let access=reply.access_token.filter(|s|!s.is_empty()).ok_or("Google no entregó acceso")?;
    if let Some(replacement)=reply.refresh_token.filter(|s|!s.is_empty()&&s!=previous){
        persist(&replacement)?;
    }
    Ok(access)
}
pub(crate) fn token(client_id:&str)->Result<String,String>{
    let refresh=vault()?.get_password().map_err(|_|"No hay autorización guardada. Conectá Google Drive desde Ajustes.")?;
    let result=exchange(&[("client_id",client_id),("grant_type","refresh_token"),("refresh_token",&refresh)])?;
    access_from_refresh_reply(result,&refresh,|replacement|
        vault()?.set_password(replacement).map_err(|_|"Windows no pudo proteger la nueva autorización de Google".into()))
}
fn challenge(v:&str)->String{URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes()))}
fn verifier()->(String,String,String){
    let random=format!("{}{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple());
    let state=uuid::Uuid::new_v4().simple().to_string();
    let hash=challenge(&random);
    (random,hash,state)
}
#[derive(Debug,PartialEq,Eq)]
enum Callback {
    Code(String),
    Denied,
    Ignored,
}
/// Accept exactly one state and authorization code. Unexpected callback
/// paths, duplicate query fields and unsolicited loopback traffic must not
/// obtain a token exchange, nor stop the legitimate OAuth attempt.
fn parse_callback(target:&str,expected:&str)->Callback{
    if !target.starts_with("/callback?"){return Callback::Ignored;}
    let Ok(url)=Url::parse(&format!("http://localhost{target}")) else{return Callback::Ignored};
    if url.path()!="/callback"{return Callback::Ignored;}
    let (mut state,mut code,mut error)=(None,None,None);
    for (k,v) in url.query_pairs(){
        match k.as_ref(){
            "state"=>{if state.replace(v.into_owned()).is_some(){return Callback::Ignored;}},
            "code"=>{if code.replace(v.into_owned()).is_some(){return Callback::Ignored;}},
            "error"=>{if error.replace(v.into_owned()).is_some(){return Callback::Ignored;}},
            _=>{},
        }
    }
    if state.as_deref()!=Some(expected){return Callback::Ignored;}
    if error.is_some(){return Callback::Denied;}
    match code{
        Some(value) if !value.is_empty()&&value.len()<=2048&&
            !value.chars().any(|c|c.is_control()||c.is_whitespace())=>Callback::Code(value),
        _=>Callback::Ignored,
    }
}
fn wait_callback(listener:TcpListener,expected:&str)->Result<String,String>{
    listener.set_nonblocking(true).map_err(|_|"No se pudo escuchar el callback de Google")?;
    let deadline=Instant::now()+Duration::from_secs(180);
    while Instant::now()<deadline{
        match listener.accept(){
            Ok((mut sock,peer))=>{
                if !peer.ip().is_loopback(){continue;}
                if sock.set_read_timeout(Some(Duration::from_secs(3))).is_err(){continue;}
                let mut buf=[0u8;8192];
                let n=match sock.read(&mut buf){Ok(n)=>n,Err(_)=>continue};
                let line=match std::str::from_utf8(&buf[..n]){
                    Ok(value)=>value.lines().next().unwrap_or(""),
                    Err(_)=>continue,
                };
                let target=line.strip_prefix("GET ").and_then(|r|r.split(' ').next()).unwrap_or("");
                let callback=parse_callback(target,expected);
                let response=match &callback{
                    Callback::Code(_)=>
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n<html><body>Autorización recibida. Volvé a AutoTrace para confirmar la conexión.</body></html>",
                    Callback::Denied=>
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\nAutorización cancelada",
                    Callback::Ignored=>
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\nSolicitud no válida",
                };
                let _=sock.write_all(response.as_bytes());
                match callback{
                    Callback::Code(code)=>return Ok(code),
                    Callback::Denied=>return Err("Google canceló la autorización. No se guardaron credenciales.".into()),
                    Callback::Ignored=>continue,
                }
            },
            Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(100)),
            Err(_)=>return Err("No se pudo recibir autorización OAuth".into())
        }
    }
    Err("Expiró el tiempo para conectar la cuenta Google".into())
}
pub(crate) fn connect(client_id:&str)->Result<(),String>{
    if !client_id_valid(client_id){return Err("Falta un Client ID OAuth válido tipo Desktop".into());}
    let listener=TcpListener::bind("127.0.0.1:0").map_err(|_|"No se pudo escuchar un puerto local OAuth")?;
    let redirect=format!("http://127.0.0.1:{}/callback",listener.local_addr().map_err(|_|"Puerto OAuth inválido")?.port());
    let (proof,hash,state)=verifier();
    let mut url=Url::parse("https://accounts.google.com/o/oauth2/v2/auth").map_err(|e|e.to_string())?;
    url.query_pairs_mut().append_pair("client_id",client_id)
        .append_pair("redirect_uri",&redirect).append_pair("response_type","code")
        .append_pair("scope",SCOPE).append_pair("code_challenge",&hash)
        .append_pair("code_challenge_method","S256").append_pair("access_type","offline")
        .append_pair("state",&state).append_pair("prompt","consent");
    webbrowser::open(url.as_str()).map_err(|_|"No se pudo abrir Google en el navegador")?;
    let code=wait_callback(listener,&state)?;
    let res=exchange(&[("client_id",client_id),("grant_type","authorization_code"),
        ("code",&code),("redirect_uri",&redirect),("code_verifier",&proof)])?;
    let refresh=res.refresh_token.filter(|s|!s.is_empty())
        .ok_or("Google no entregó un token de renovación para acceso sin conexión")?;
    vault()?.set_password(&refresh).map_err(|_|"Windows no permitió almacenar el token de forma segura")?;
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn pkce_state_and_scope_are_limited(){
        let (v,h,s)=verifier();
        assert_eq!(v.len(),64);
        assert_eq!(h,challenge(&v));
        assert_eq!(h.len(),43);
        assert_eq!(s.len(),32);
        assert_eq!(SCOPE,"https://www.googleapis.com/auth/drive.file");
    }
    #[test] fn oauth_callback_rejects_wrong_state_duplicate_codes_and_handles_denial(){
        let state="synthetic-state-abc";
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&code=abc-123",state),
                   Callback::Code("abc-123".into()));
        assert_eq!(parse_callback("/callback?state=wrong&code=abc-123",state),Callback::Ignored);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&code=1&code=2",state),Callback::Ignored);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&state=synthetic-state-abc&code=1",state),Callback::Ignored);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&code=",state),Callback::Ignored);
        assert_eq!(parse_callback("/other?state=synthetic-state-abc&code=abc",state),Callback::Ignored);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&error=access_denied",state),Callback::Denied);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&code=abc&error=access_denied",state),Callback::Denied);
        assert_eq!(parse_callback("/callback?state=synthetic-state-abc&code=a%0Ab",state),Callback::Ignored);
    }
    #[test] fn refresh_token_rotation_is_saved_before_access_and_fails_closed_on_vault_error(){
        let reply=Tokens{access_token:Some("synthetic-access-1".into()),
            refresh_token:Some("synthetic-rotated-refresh-2".into())};
        let mut written=Vec::new();
        let access=access_from_refresh_reply(reply,"synthetic-old-refresh",|new|{
            written.push(new.to_owned());Ok(())
        }).unwrap();
        assert_eq!(access,"synthetic-access-1");
        assert_eq!(written,vec!["synthetic-rotated-refresh-2"]);
        let failure=access_from_refresh_reply(
            Tokens{access_token:Some("synthetic-access-1".into()),refresh_token:Some("replacement".into())},
            "old",|_|Err("synthetic vault failure".into()));
        assert!(failure.is_err(),"the access token must not escape if credential persistence failed");
        let mut writes=0;
        let stable=access_from_refresh_reply(
            Tokens{access_token:Some("stable-access".into()),refresh_token:Some("old".into())},
            "old",|_|{writes+=1;Ok(())}).unwrap();
        assert_eq!(stable,"stable-access");
        assert_eq!(writes,0,"unchanged token must not rewrite Windows vault");
        assert!(access_from_refresh_reply(Tokens{access_token:None,refresh_token:None},
            "old",|_|Ok(())).is_err());
    }
    #[test] fn oauth_refresh_http_rejects_provider_errors_and_malformed_responses(){
        use std::{io::{Read,Write},net::TcpListener};
        for (status,body,accepted) in [
            (200,r#"{"access_token":"test-access","refresh_token":"test-rotated"}"#,true),
            (401,r#"{"error":"invalid_grant"}"#,false),
            (200,"THIS IS NOT JSON",false),
            (200,r#"{"refresh_token":"rotated-but-no-access"}"#,false),
        ]{
            let listener=TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint=format!("http://{}/mock-token",listener.local_addr().unwrap());
            let server=std::thread::spawn(move||{
                let (mut socket,_)=listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request=Vec::new();
                let mut buf=[0u8;2048];
                loop{
                    let n=socket.read(&mut buf).unwrap();
                    assert!(n>0,"mock provider request disconnected");
                    request.extend_from_slice(&buf[..n]);
                    let Some(head)=request.windows(4).position(|w|w==b"\r\n\r\n") else{continue;};
                    let headers=String::from_utf8_lossy(&request[..head]).to_ascii_lowercase();
                    let len=headers.lines().find_map(|line|
                        line.strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok()))
                        .unwrap_or(0);
                    if request.len()>=head+4+len{break;}
                }
                write!(socket,"HTTP/1.1 {status} TEST\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            });
            let fields=[("grant_type","refresh_token"),("refresh_token","synthetic-only")];
            let result=exchange_with(&http().unwrap(),&endpoint,&fields)
                .and_then(|response|access_from_refresh_reply(response,"synthetic-only",|_|Ok(())));
            assert_eq!(result.is_ok(),accepted,"mock OAuth status {status} must never fabricate access");
            server.join().unwrap();
        }
    }
    #[test] fn vault_disconnect_missing_credential_is_idempotent(){
        assert!(finish_forget(Ok(())).is_ok());
        assert!(finish_forget(Err(keyring::Error::NoEntry)).is_ok());
    }
    #[test] fn rejects_wrong_oauth_client(){
        assert!(client_id_valid("012345678901-abc.apps.googleusercontent.com"));
        assert!(!client_id_valid("https://evil.invalid"));
    }
}