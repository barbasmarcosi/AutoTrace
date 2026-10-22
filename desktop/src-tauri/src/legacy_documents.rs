//! Compatibility reader for the customer's actual one-sheet "Presupuesto" and
//! "Remito trabajo" templates. Does not import the logo or data into GitHub.
//! Parsing is deliberately fail-closed on unknown layouts.
use calamine::{Data, Reader, Xlsx};
use chrono::{Duration,NaiveDate};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::{fs,io::Cursor};

fn error(e:impl std::fmt::Display)->String{format!("No se pudo interpretar el documento: {e}")}
#[derive(Debug,Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct LegacyItem {
    pub id:String,
    pub kind:String,
    pub description:String,
    pub quantity:f64,
    pub unit_price:f64,
}
#[derive(Debug,Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct LegacyDocument {
    pub document_kind:String,
    pub document_number:String,
    pub date:String,
    pub workshop:String,
    pub customer_name:String,
    pub customer_phone:String,
    pub customer_address:String,
    pub customer_email:String,
    pub customer_notes:String,
    pub make:String,
    pub model:String,
    pub plate:String,
    pub vin:String,
    pub mileage:f64,
    pub vehicle_observation:String,
    pub items:Vec<LegacyItem>,
    pub total:f64,
    pub source_fingerprint:String,
}
fn value<'a>(sheet:&'a calamine::Range<Data>,row:u32,column:u32)->&'a Data {
    sheet.get_value((row-1,column-1)).unwrap_or(&Data::Empty)
}
fn text(c:&Data)->String{
    match c{Data::String(s)=>s.trim().to_string(),Data::Int(i)=>i.to_string(),
      Data::Float(v)=>v.to_string(),Data::Bool(v)=>v.to_string(),Data::Empty=>String::new(),
      _=>c.to_string().trim().to_string()}
}
fn amount(c:&Data)->Result<f64,String>{
    let n=match c{
      Data::Empty=>0.0,Data::Int(i)=>*i as f64,Data::Float(f)=>*f,
      _=>return Err(format!("Se esperaba número y apareció {}",text(c)))
    };
    if !n.is_finite()||n<0.0||n>1e12{return Err("Importe fuera de rango".into());}
    Ok((n*100.0).round()/100.0)
}
fn date(c:&Data)->Result<String,String>{
    let serial=match c {
       Data::Int(i)=>*i as f64,
       Data::Float(f)=>*f,
       Data::DateTime(t)=>t.as_f64(),
       Data::Empty=>return Err("Fecha del documento vacía".into()),
       Data::String(s)=>{
           if NaiveDate::parse_from_str(s,"%Y-%m-%d").is_ok(){return Ok(s.clone());}
           return Err("Fecha de documento desconocida".into());
       }
       _=>return Err("Formato de fecha no soportado".into())
    };
    if !serial.is_finite() || !(1.0..=150000.0).contains(&serial) {return Err("Fecha Excel inválida".into());}
    let base=NaiveDate::from_ymd_opt(1899,12,30).ok_or("Error de calendario")?;
    Ok((base+Duration::days(serial.floor() as i64)).format("%Y-%m-%d").to_string())
}
pub(crate) fn parse(bytes:&[u8])->Result<LegacyDocument,String>{
    if bytes.len()>15_000_000 || bytes.len()<250 {return Err("El archivo excede 15 MB o está vacío".into());}
    let mut wb:Xlsx<_>=Xlsx::new(Cursor::new(bytes)).map_err(error)?;
    if wb.sheet_names().len()!=1{return Err("La plantilla debe contener una sola hoja".into());}
    let sheet_name=wb.sheet_names()[0].clone();
    let formulas=wb.worksheet_formula(&sheet_name).map_err(error)?;
    for (row,col,formula) in formulas.used_cells(){
        if col!=9 || row<19 || row>48 || formula.to_ascii_lowercase().contains("http") || formula.contains('['){
            return Err("La plantilla contiene fórmulas inesperadas".into());
        }
    }
    let sheet=wb.worksheet_range(&sheet_name).map_err(error)?;
    let heading=text(value(&sheet,1,2)).to_uppercase();
    let (kind,total_row,last_line)=
        if heading.contains("PRESUPUESTO") {( "quote",47,45 )}
        else if heading.contains("REMITO TRABAJO") {("work",46,44)}
        else{return Err("Solo se admite el formato Presupuesto o Remito Trabajo del taller".into());};
    if !text(value(&sheet,18,2)).to_uppercase().contains("REFERENCIA")
      || !text(value(&sheet,18,8)).to_uppercase().contains("CANTIDAD")
      || !text(value(&sheet,18,9)).to_uppercase().contains("UNITARIO"){
        return Err("Encabezados de detalle incompatibles".into());
    }
    if !text(value(&sheet,total_row,9)).to_uppercase().contains("TOTAL"){
        return Err("No se encontró el total del documento".into());
    }
    let mut rows:Vec<LegacyItem>=Vec::new();
    for row in 20..=last_line {
        let description=text(value(&sheet,row,2));
        let qty_cell=value(&sheet,row,8);let price_cell=value(&sheet,row,9);
        let has_qty=!matches!(qty_cell,Data::Empty);
        let has_price=!matches!(price_cell,Data::Empty);
        if description.is_empty(){
            if has_qty || has_price {return Err(format!("Fila {row}: cantidades sin descripción"));}
            continue;
        }
        if !has_qty && !has_price {
            // Merged multiline descriptions may continue on the next row.
            let previous=rows.last_mut().ok_or_else(||format!("Fila {row}: texto sin un renglón anterior"))?;
            previous.description.push('\n');previous.description.push_str(&description);
            continue;
        }
        if !has_qty || !has_price{return Err(format!("Fila {row}: falta cantidad o precio unitario"));}
        let qty=amount(qty_cell)?;
        let unit=amount(price_cell)?;
        if qty<=0.0||qty>100000.0{return Err(format!("Fila {row}: cantidad inválida"));}
        let total=(qty*unit*100.0).round()/100.0;
        if total>1e12{return Err(format!("Fila {row}: importe excesivo"));}
        let shown=amount(value(&sheet,row,10))?;
        if (shown-total).abs()>0.011{return Err(format!("Fila {row}: el subtotal no coincide"));}
        let d=description.to_uppercase();
        let kind=if d.contains("MANO DE OBRA") || d.contains("M. O.") {"labor"} else {"part"};
        rows.push(LegacyItem{id:format!("item-{row}"),kind:kind.into(),description,quantity:qty,unit_price:unit});
    }
    if rows.is_empty(){return Err("El documento no contiene renglones de trabajo".into());}
    let actual=rows.iter().map(|x|(x.quantity*x.unit_price*100.0).round() as i64).sum::<i64>();
    let total=amount(value(&sheet,total_row,10))?;
    if (actual as f64/100.0-total).abs()>0.011{return Err("El total no coincide con los renglones del documento".into());}
    let observation=text(value(&sheet,16,4));
    let mileage=match value(&sheet,16,4){Data::Int(_) | Data::Float(_)=>amount(value(&sheet,16,4))?,_=>0.0};
    Ok(LegacyDocument {
        document_kind:kind.into(),
        document_number:text(value(&sheet,4,4)),
        date:date(value(&sheet,4,10))?,
        workshop:text(value(&sheet,6,2)),
        customer_name:text(value(&sheet,6,9)),
        customer_phone:text(value(&sheet,7,9)),
        customer_address:text(value(&sheet,8,9)),
        customer_email:text(value(&sheet,9,9)),
        customer_notes:text(value(&sheet,10,9)),
        make:text(value(&sheet,12,4)),
        model:text(value(&sheet,13,4)),
        plate:text(value(&sheet,14,4)),
        vin:text(value(&sheet,15,4)),
        mileage,
        vehicle_observation:observation,
        items:rows,
        total,
        source_fingerprint:format!("{:x}",Sha256::digest(bytes)),
    })
}
#[tauri::command]
pub(crate) fn choose_legacy_document()->Result<Option<LegacyDocument>,String>{
    let Some(path)=rfd::FileDialog::new().set_title("Presupuesto o remito Excel existente")
      .add_filter("Excel de taller",&["xlsx"]).pick_file() else{return Ok(None)};
    let file=fs::read(&path).map_err(error)?;
    parse(&file).map(Some)
}

/// Export in the same cell layout as the customer's document templates.
/// Importing a PDF, the spreadsheet header/footer VML logo, and full workbook
/// formatting are deliberately outside this first compatibility increment.
pub(crate) fn generate(data:&crate::store::AppData,order_id:&str,kind:&str)->Result<Vec<u8>,String>{
    generate_with_logo(data,order_id,kind,None)
}
fn generate_with_logo(data:&crate::store::AppData,order_id:&str,kind:&str,logo_path:Option<&std::path::Path>)->Result<Vec<u8>,String>{
    crate::store::validate(data)?;
    if !matches!(kind,"quote"|"work"){return Err("Tipo de documento no admitido".into());}
    let v=serde_json::to_value(data).map_err(error)?;
    let order=v["orders"].as_array().ok_or("Trabajos inválidos")?.iter()
        .find(|x|x["id"]==order_id).ok_or("Trabajo inexistente")?;
    let vehicle=v["vehicles"].as_array().ok_or("Vehículos inválidos")?.iter()
        .find(|x|x["id"]==order["vehicleId"]).ok_or("Vehículo inexistente")?;
    let client=v["clients"].as_array().ok_or("Clientes inválidos")?.iter()
        .find(|x|x["id"]==vehicle["clientId"]).ok_or("Cliente inexistente")?;
    let items=order["items"].as_array().ok_or("Este trabajo no tiene renglones. Editalo y cargá el detalle antes de exportar.")?;
    if items.is_empty()||items.len()>26{return Err("El documento debe tener entre 1 y 26 renglones".into());}
    let agreed=order["charged"].as_f64().ok_or("Importe acordado inválido")?;
    let mut wb=rust_xlsxwriter::Workbook::new();
    let sheet=wb.add_worksheet();
    sheet.set_name("Hoja1").map_err(error)?;
    if let Some(path)=logo_path {
        let image=rust_xlsxwriter::Image::new(path).map_err(error)?
            .set_scale_height(0.4).set_scale_width(0.4);
        sheet.set_header("&C&[Picture]");
        sheet.set_header_image(&image,rust_xlsxwriter::HeaderImagePosition::Center).map_err(error)?;
    }
    let header=rust_xlsxwriter::Format::new().set_bold().set_font_size(17.0);
    let emphasis=rust_xlsxwriter::Format::new().set_bold();
    sheet.set_column_width(0,3).map_err(error)?;
    for c in 1..=6{sheet.set_column_width(c,14).map_err(error)?;}
    sheet.set_column_width(7,13).map_err(error)?;
    sheet.set_column_width(8,19).map_err(error)?;
    sheet.set_column_width(9,19).map_err(error)?;
    let title=if kind=="quote"{"PRESUPUESTO - ESTIMACION REPARACION"}else{"REMITO TRABAJO"};
    sheet.merge_range(0,1,1,9,title,&header).map_err(error)?;
    sheet.write_string(3,1,if kind=="quote"{"N° Presupuesto:"}else{"N° Remito:"}).map_err(error)?;
    sheet.write_string(3,7,if kind=="quote"{"Fecha Creación:"}else{"Fecha Trabajo:"}).map_err(error)?;
    let day=NaiveDate::parse_from_str(order["openedAt"].as_str().ok_or("Fecha faltante")?,"%Y-%m-%d").map_err(error)?;
    let base=NaiveDate::from_ymd_opt(1899,12,30).ok_or("Fecha base inválida")?;
    sheet.write_number_with_format(3,9,(day-base).num_days() as f64,
        &rust_xlsxwriter::Format::new().set_num_format("dd/mm/yyyy")).map_err(error)?;
    sheet.write_string_with_format(5,1,v["settings"]["garageName"].as_str().unwrap_or("Mi taller"),&emphasis).map_err(error)?;
    for (row,label,value) in [
        (5,"Cliente:",client["name"].as_str().unwrap_or("")),
        (6,"Teléfono:",client["phone"].as_str().unwrap_or("")),
        (7,"Dirección:",""),
        (8,"Mail:",client["email"].as_str().unwrap_or("")),
        (9,"Observación:","")
    ]{
        sheet.write_string(row,7,label).map_err(error)?;
        if !value.is_empty(){sheet.write_string(row,8,value).map_err(error)?;}
    }
    for(row,label,val) in [
       (11,"MARCA:",vehicle["make"].as_str().unwrap_or("")),
       (12,"MODELO:",vehicle["model"].as_str().unwrap_or("")),
       (13,"MATRICULA:",vehicle["plate"].as_str().unwrap_or("")),
       (14,"VIN:",vehicle["vin"].as_str().unwrap_or(""))
    ]{
        sheet.write_string(row,1,label).map_err(error)?;
        if !val.is_empty(){sheet.write_string(row,3,val).map_err(error)?;}
    }
    sheet.write_string(15,1,"OBSERVACION:").map_err(error)?;
    sheet.write_number(15,3,order["mileage"].as_f64().unwrap_or(0.0)).map_err(error)?;
    for (column,label) in [(1,"REFERENCIA / DETALLE PIEZAS"),(7,"CANTIDAD"),(8,"VALOR UNITARIO"),(9,"TOTAL")]{
        sheet.write_string_with_format(17,column,label,&emphasis).map_err(error)?;
    }
    let mut sum=0.0;
    for(i,item)in items.iter().enumerate(){
        let row=19+i as u32;
        let description=item["description"].as_str().ok_or("Renglón sin descripción")?;
        let q=item["quantity"].as_f64().ok_or("Cantidad inválida")?;
        let u=item["unitPrice"].as_f64().ok_or("Unitario inválido")?;
        if q<=0.0||u<0.0{return Err("Valores negativos en renglón".into());}
        let subtotal=(q*u*100.0).round()/100.0;
        sheet.write_string(row,1,description).map_err(error)?;
        sheet.write_number(row,7,q).map_err(error)?;
        sheet.write_number(row,8,u).map_err(error)?;
        sheet.write_number(row,9,subtotal).map_err(error)?;
        sum+=subtotal;
        if description.contains('\n'){sheet.set_row_height(row,44).map_err(error)?;}
    }
    let sum=(sum*100.0).round()/100.0;
    if (sum-agreed).abs()>0.011{return Err("El detalle no coincide con el importe acordado. Corregí el trabajo antes de exportar.".into());}
    let total=if kind=="quote"{46}else{45};
    sheet.write_string_with_format(total,8,"TOTAL",&emphasis).map_err(error)?;
    sheet.write_number_with_format(total,9,sum,&emphasis).map_err(error)?;
    sheet.write_string(total,1,if kind=="quote"{"Presupuesto sujeto a revisión. Validez 15 días."}else{"Detalle de trabajo realizado."}).map_err(error)?;
    wb.save_to_buffer().map_err(error)
}
fn logo_folder(app:&tauri::AppHandle)->Result<std::path::PathBuf,String>{
    use tauri::Manager;
    let folder=app.path().app_local_data_dir().map_err(error)?.join("branding");
    fs::create_dir_all(&folder).map_err(error)?;
    Ok(folder)
}
fn existing_logo(app:&tauri::AppHandle)->Result<Option<std::path::PathBuf>,String>{
    let folder=logo_folder(app)?;
    for ext in ["png","jpg","jpeg"]{
        let path=folder.join(format!("workshop-logo.{ext}"));
        if path.is_file(){return Ok(Some(path));}
    }
    Ok(None)
}
#[tauri::command]
pub(crate) fn choose_workshop_logo(app:tauri::AppHandle)->Result<Option<String>,String>{
    let Some(path)=rfd::FileDialog::new().set_title("Logo del taller")
        .add_filter("Imagen PNG o JPEG",&["png","jpg","jpeg"]).pick_file() else{return Ok(None)};
    let ext=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(ext.as_str(),"png"|"jpg"|"jpeg"){return Err("Logo: formato inválido".into());}
    let size=fs::metadata(&path).map_err(error)?.len();
    if size>5_000_000||size<16{return Err("Logo demasiado grande o vacío".into());}
    rust_xlsxwriter::Image::new(&path).map_err(error)?;
    let folder=logo_folder(&app)?;
    let target=folder.join(format!("workshop-logo.{ext}"));
    let temp=folder.join(format!(".new-logo-{}.tmp",uuid::Uuid::new_v4()));
    fs::copy(&path,&temp).map_err(error)?;
    let check=fs::metadata(&temp).map_err(error)?.len();
    if check!=size{let _=fs::remove_file(&temp);return Err("No se pudo copiar todo el logo".into());}
    // Keep a working old logo in case the new file cannot be published.
    if target.exists(){fs::remove_file(&target).map_err(error)?;}
    fs::rename(&temp,&target).map_err(error)?;
    for other in ["png","jpg","jpeg"]{
        if other!=ext{let _=fs::remove_file(folder.join(format!("workshop-logo.{other}")));}
    }
    Ok(Some("Logo configurado localmente. Se agregará al encabezado impreso.".into()))
}

#[tauri::command]
pub(crate) fn export_legacy_document(app:tauri::AppHandle,order_id:String,kind:String)->Result<bool,String>{
    let data=crate::store::read(&crate::store::open_app(&app)?)?.data;
    let configured=existing_logo(&app)?;
    let bytes=generate_with_logo(&data,&order_id,&kind,configured.as_deref())?;
    // Self-check: a generated customer document must parse back to identical lines.
    let checked=parse(&bytes)?;
    if checked.document_kind!=kind{return Err("Error al verificar el documento generado".into());}
    let base=if kind=="quote"{"AutoTrace_presupuesto.xlsx"}else{"AutoTrace_remito.xlsx"};
    let Some(path)=rfd::FileDialog::new().set_file_name(base).add_filter("Documento Excel",&["xlsx"]).save_file()
        else{return Ok(false)};
    if path.exists(){return Err("Para no pisar un documento, elegí un nombre nuevo".into());}
    let parent=path.parent().ok_or("Carpeta inválida")?;
    let temp=parent.join(format!(".autotrace-export-{}.tmp",uuid::Uuid::new_v4()));
    let result=(||{
        fs::write(&temp,&bytes).map_err(error)?;
        let check=fs::read(&temp).map_err(error)?;
        if check!=bytes{return Err("Error de integridad al guardar documento".into());}
        fs::rename(&temp,&path).map_err(error)?;
        Ok(true)
    })();
    if result.is_err(){let _=fs::remove_file(temp);}
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::Workbook;
    fn fixture(quote:bool)->Vec<u8>{
        let mut w=Workbook::new();let s=w.add_worksheet();
        s.set_name("Hoja1").unwrap();
        s.write_string(0,1,if quote{"PRESUPUESTO - ESTIMACION REPARACION"}else{"REMITO TRABAJO"}).unwrap();
        s.write_string(3,1,if quote{"N° Presupuesto:"}else{"N° Remito:"}).unwrap();
        s.write_number(3,9,46317.0).unwrap();
        s.write_string(5,1,"Taller ficticio").unwrap();
        s.write_string(5,8,"CLIENTE FICTICIO").unwrap();
        s.write_string(11,3,"Toyota").unwrap();s.write_string(12,3,"Hilux").unwrap();
        s.write_string(13,3,"AF012BC").unwrap();
        s.write_string(17,1,"REFERENCIA / DETALLE PIEZAS").unwrap();
        s.write_string(17,7,"CANTIDAD").unwrap();
        s.write_string(17,8,"VALOR UNITARIO").unwrap();
        s.write_string(19,1,"FILTRO").unwrap();s.write_number(19,7,2.0).unwrap();
        s.write_number(19,8,1200.0).unwrap();s.write_number(19,9,2400.0).unwrap();
        s.write_string(20,1,"MANO DE OBRA - diagnóstico").unwrap();
        s.write_number(20,7,1.0).unwrap();s.write_number(20,8,2000.0).unwrap();
        s.write_number(20,9,2000.0).unwrap();
        s.write_string(21,1,"y control de bujías").unwrap();
        s.write_string(if quote{46}else{45},8,"TOTAL").unwrap();
        s.write_number(if quote{46}else{45},9,4400.0).unwrap();
        w.save_to_buffer().unwrap()
    }
    #[test]
    fn both_templates_keep_line_quantities_and_continuation(){
        for quote in [true,false]{
            let parsed=parse(&fixture(quote)).unwrap();
            assert_eq!(parsed.document_kind,if quote{"quote"}else{"work"});
            assert_eq!(parsed.date,"2026-10-22");
            assert_eq!(parsed.items.len(),2);
            assert_eq!(parsed.items[0].quantity,2.0);
            assert_eq!(parsed.items[1].kind,"labor");
            assert!(parsed.items[1].description.contains("control de bujías"));
            assert_eq!(parsed.total,4400.0);
            assert_eq!(parsed.plate,"AF012BC");
        }
    }
    #[test]
    fn export_existing_client_work_to_both_legacy_formats_roundtrips(){
        let raw=serde_json::json!({
         "clients":[{"id":"c1","name":"CLIENTE PRUEBA","phone":"00123","email":"","notes":"","createdAt":""}],
         "vehicles":[{"id":"v1","clientId":"c1","plate":"AF012BC","make":"Toyota","model":"Hilux","year":"","mileage":32450,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
         "orders":[{"id":"o1","vehicleId":"v1","title":"Service","category":"Mecánica","status":"done",
             "openedAt":"2026-10-22","completedAt":"","mileage":32450,"description":"","partsCost":0,
             "laborCost":0,"charged":4400,"paid":0,"notes":"","createdAt":"",
             "items":[{"id":"item1","kind":"part","description":"FILTRO","quantity":2,"unitPrice":1200},
                      {"id":"item2","kind":"labor","description":"MANO DE OBRA - diagnóstico\ncontrol de bujías","quantity":1,"unitPrice":2000}]}],
         "settings":{"garageName":"TALLER PRUEBA","currency":"ARS","demo":false}
        });
        let data:crate::store::AppData=serde_json::from_value(raw).unwrap();
        for kind in ["quote","work"]{
            let xlsx=generate(&data,"o1",kind).unwrap();
            let document=parse(&xlsx).unwrap();
            assert_eq!(document.document_kind,kind);
            assert_eq!(document.customer_name,"CLIENTE PRUEBA");
            assert_eq!(document.plate,"AF012BC");
            assert_eq!(document.items.len(),2);
            assert_eq!(document.items[0].quantity,2.);
            assert_eq!(document.items[1].unit_price,2000.);
            assert!(document.items[1].description.contains("control de bujías"));
            assert_eq!(document.total,4400.);
        }
    }
    #[test]
    fn refuses_unrecognized_file_without_creating_records(){assert!(parse(b"not excel").is_err());}
}
