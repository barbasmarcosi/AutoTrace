//! Safe, explicit Windows save dialog for an offline, customer-facing PDF.
//! The frontend creates only the scoped document. Rust never sends it to a
//! network provider and only writes to the user-selected path.
use std::{fs,io::Write,path::Path};

fn validate_payload(bytes:&[u8],filename:&str)->Result<(),String>{
    if bytes.len()<120 || bytes.len()>5_000_000 ||
       !bytes.starts_with(b"%PDF-1.4\n") ||
       !bytes.ends_with(b"%%EOF\n"){
        return Err("Documento PDF inválido o demasiado grande".into());
    }
    if filename.len()>140 || filename.len()<16 ||
       !filename.starts_with("AutoTrace_") ||
       !filename.ends_with(".pdf") ||
       !filename.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'-'|'_'|'.')){
        return Err("Nombre de exportación PDF inválido".into());
    }
    Ok(())
}
fn save_file(bytes:&[u8],path:&Path)->Result<(),String>{
    if !path.extension().is_some_and(|x|x.eq_ignore_ascii_case("pdf")){
        return Err("Seleccioná un archivo con extensión .pdf".into());
    }
    // Create a temporary file in the destination directory and sync before
    // promotion, so a failed write never truncates an existing PDF.
    let parent=path.parent().ok_or("Carpeta de destino inválida")?;
    let id=uuid::Uuid::new_v4();
    let temporary=parent.join(format!(".autotrace-pdf-{id}.tmp"));
    let result=(||{
        let mut writer=fs::OpenOptions::new().write(true).create_new(true)
            .open(&temporary).map_err(|_|"No se puede crear el PDF en esa carpeta")?;
        writer.write_all(bytes).map_err(|_|"No se pudo escribir el PDF completo")?;
        writer.sync_all().map_err(|_|"No se pudo confirmar el PDF en disco")?;
        drop(writer);
        // On Windows rename does not replace an existing destination. Do not
        // delete existing reports automatically; require a distinct filename.
        if path.exists(){return Err("El archivo PDF ya existe. Elegí otro nombre para no sobrescribirlo".into());}
        fs::rename(&temporary,path).map_err(|_|"No se pudo publicar el PDF")?;
        Ok(())
    })();
    if result.is_err(){let _=fs::remove_file(temporary);}
    result
}
#[tauri::command]
pub(crate) fn save_customer_pdf(bytes:Vec<u8>,filename:String)->Result<bool,String>{
    validate_payload(&bytes,&filename)?;
    let Some(path)=rfd::FileDialog::new()
        .set_title("Guardar presupuesto o remito PDF")
        .add_filter("Documento PDF",&["pdf"])
        .set_file_name(&filename)
        .save_file()
    else {return Ok(false);};
    save_file(&bytes,&path)?;
    Ok(true)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn fail_closed_on_bad_document_or_name(){
        let mut valid=b"%PDF-1.4\n".to_vec();valid.extend(vec![b'A';200]);valid.extend(b"%%EOF\n");
        assert!(validate_payload(&valid,"AutoTrace_presupuesto_AF012BC.pdf").is_ok());
        assert!(validate_payload(b"%PDF-1.4\nshort%%EOF\n","AutoTrace_remito_AF012BC.pdf").is_err());
        assert!(validate_payload(&valid,"../escape.pdf").is_err());
        assert!(validate_payload(&valid,"AutoTrace_salida.xlsx").is_err());
        assert!(validate_payload(&vec![b'X';5_000_001],"AutoTrace_remito_AF012BC.pdf").is_err());
    }
    #[test]
    fn existing_documents_must_not_be_overwritten_or_truncated(){
        let dir=std::env::temp_dir().join(format!("autotrace-pdf-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let target=dir.join("out.pdf");
        fs::write(&target,b"original customer PDF").unwrap();
        assert!(save_file(b"new", &target).is_err());
        assert_eq!(fs::read(&target).unwrap(),b"original customer PDF");
        let other=dir.join("different.pdf");
        save_file(b"new",&other).unwrap();
        assert_eq!(fs::read(&other).unwrap(),b"new");
        fs::remove_dir_all(dir).unwrap();
    }
}
