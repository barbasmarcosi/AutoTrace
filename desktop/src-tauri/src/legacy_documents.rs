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
    fn refuses_unrecognized_file_without_creating_records(){assert!(parse(b"not excel").is_err());}
}
