//! Strict XLSX interchange format. Backups remain private and are not customer reports.
//! No formulas, macros or implicit Excel date coercion are accepted.
use crate::store::{validate, AppData};
use calamine::{Data, DataType, Reader, Xlsx};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const CLIENTS: &[(&str, &str)] = &[("id","id"),("nombre","name"),("telefono","phone"),("email","email"),("notas","notes"),("creado_el","createdAt")];
const VEHICLES: &[(&str,&str)] = &[("id","id"),("cliente_id","clientId"),("patente","plate"),("marca","make"),("modelo","model"),("anio","year"),("kilometraje","mileage"),("vin","vin"),("color","color"),("proximo_servicio","nextServiceDate"),("notas","notes"),("creado_el","createdAt")];
const ORDERS: &[(&str,&str)] = &[("id","id"),("vehiculo_id","vehicleId"),("titulo","title"),("categoria","category"),("estado","status"),("fecha_apertura","openedAt"),("fecha_cierre","completedAt"),("kilometraje","mileage"),("descripcion","description"),("costo_repuestos","partsCost"),("costo_mano_obra","laborCost"),("importe_acordado","charged"),("total_pagado","paid"),("notas","notes"),("creado_el","createdAt")];
const SETTINGS: &[(&str,&str)] = &[("nombre_taller","garageName"),("moneda","currency"),("datos_demostracion","demo")];

pub(crate) fn canonical_bytes(data: &AppData) -> Result<Vec<u8>, String> {
    let mut v = serde_json::to_value(data).map_err(|e| e.to_string())?;
    for k in ["clients", "vehicles", "orders"] {
        let a = v[k].as_array_mut().ok_or("Lista inválida")?;
        a.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    }
    serde_json::to_vec(&v).map_err(|e| e.to_string())
}
fn checksum(data: &AppData) -> Result<String,String> {
    Ok(format!("{:x}", Sha256::digest(canonical_bytes(data)?)))
}
fn xlsx_err(e: impl std::fmt::Display) -> String {format!("Error en el archivo Excel: {e}")}

fn emit_sheet(workbook: &mut Workbook, title: &str, columns: &[(&str,&str)], rows: &[Value]) -> Result<(),String> {
    let sheet = workbook.add_worksheet();
    sheet.set_name(title).map_err(xlsx_err)?;
    let heading = Format::new().set_bold().set_background_color("#E4EFF4");
    for (n, (header,_)) in columns.iter().enumerate() {
        sheet.write_string_with_format(0, n as u16, *header, &heading).map_err(xlsx_err)?;
        sheet.set_column_width(n as u16, if *header=="notas" || *header=="descripcion" {35} else {19}).map_err(xlsx_err)?;
    }
    for (r, item) in rows.iter().enumerate() {
        for (c, (_,key)) in columns.iter().enumerate() {
            let v = &item[*key];
            match v {
                Value::Null => (),
                Value::String(s) => {sheet.write_string((r+1) as u32,c as u16,s).map_err(xlsx_err)?;},
                Value::Number(n) => {let f = n.as_f64().ok_or("Número inválido")?; sheet.write_number((r+1) as u32,c as u16,f).map_err(xlsx_err)?;},
                Value::Bool(b) => {sheet.write_boolean((r+1) as u32,c as u16,*b).map_err(xlsx_err)?;},
                _ => return Err(format!("Campo no escalar en {title}.{key}")),
            }
        }
    }
    Ok(())
}

pub(crate) fn to_xlsx(data: &AppData) -> Result<Vec<u8>, String> {
    validate(data)?;
    let v = serde_json::to_value(data).map_err(xlsx_err)?;
    let mut wb = Workbook::new();
    let digest = checksum(data)?;
    let manifest = vec![
        ("format","autotrace-backup-xlsx".to_string()),
        ("schema_version","1".to_string()),
        ("created_at_utc",chrono::Utc::now().to_rfc3339()),
        ("snapshot_id",uuid::Uuid::new_v4().to_string()),
        ("app_version",env!("CARGO_PKG_VERSION").to_string()),
        ("source_schema_version","1".to_string()),
        ("timezone","America/Argentina/Buenos_Aires".to_string()),
        ("export_kind","full".to_string()),
        ("canonical_sha256",digest),
        ("count_clients",data_count(&v,"clients")?.to_string()),
        ("count_vehicles",data_count(&v,"vehicles")?.to_string()),
        ("count_orders",data_count(&v,"orders")?.to_string()),
    ];
    let ws = wb.add_worksheet(); ws.set_name("Manifest").map_err(xlsx_err)?;
    ws.set_column_width(0,25).map_err(xlsx_err)?; ws.set_column_width(1,72).map_err(xlsx_err)?;
    ws.write_string(0,0,"clave").map_err(xlsx_err)?;
    ws.write_string(0,1,"valor").map_err(xlsx_err)?;
    for (i,(k,v)) in manifest.iter().enumerate() {
        ws.write_string((i+1) as u32,0,*k).map_err(xlsx_err)?;
        ws.write_string((i+1) as u32,1,v).map_err(xlsx_err)?;
    }
    emit_sheet(&mut wb,"Clientes",CLIENTS,v["clients"].as_array().ok_or("Falta Clientes")?)?;
    emit_sheet(&mut wb,"Vehiculos",VEHICLES,v["vehicles"].as_array().ok_or("Falta Vehiculos")?)?;
    emit_sheet(&mut wb,"Trabajos",ORDERS,v["orders"].as_array().ok_or("Falta Trabajos")?)?;
    emit_sheet(&mut wb,"Configuracion",SETTINGS,&[v["settings"].clone()])?;
    wb.save_to_buffer().map_err(xlsx_err)
}
fn data_count(v:&Value, key:&str)->Result<usize,String> {Ok(v[key].as_array().ok_or("Listado inválido")?.len())}
fn cell_as_string(cell:&Data)->Result<String,String> {
    match cell {Data::String(s)=>Ok(s.clone()),Data::Empty=>Ok(String::new()), _=>Err("Se esperaba una celda de texto; Excel pudo alterar un ID, patente o fecha".into())}
}
fn cell_as_number(cell:&Data, field:&str)->Result<Value,String> {
    match cell {
        Data::Int(i) if *i>=0=>Ok(Value::from(*i)),
        Data::Float(f) if f.is_finite() && *f>=0.0 => serde_json::Number::from_f64(*f).map(Value::Number).ok_or_else(||format!("Número inválido: {field}")),
        Data::Empty => Ok(Value::from(0)),
        _ => Err(format!("Se esperaba número positivo en {field}")),
    }
}
fn read_rows(excel: &mut Xlsx<Cursor<&[u8]>>, name:&str, cols:&[(&str,&str)], max:usize)->Result<Vec<Value>,String>{
    let formula = excel.worksheet_formula(name).map_err(xlsx_err)?;
    if formula.used_cells().next().is_some() {return Err(format!("No se admiten fórmulas en {name}"));}
    let sheet = excel.worksheet_range(name).map_err(xlsx_err)?;
    if sheet.height() > max+1 {return Err(format!("La hoja {name} supera el límite de registros"));}
    let mut lines = sheet.rows();
    let header = lines.next().ok_or_else(||format!("Falta encabezado en {name}"))?;
    if header.len()!=cols.len() || cols.iter().enumerate().any(|(i,(h,_))|header[i].get_string()!=Some(*h)) {
        return Err(format!("Encabezados incompatibles en {name}"));
    }
    let mut out=Vec::new();
    for (r,cells) in lines.enumerate() {
        if cells.iter().all(|v|matches!(v,Data::Empty)) {return Err(format!("Fila {} completamente vacía en {name}",r+2));}
        if cells.len()>cols.len() && cells[cols.len()..].iter().any(|c|!matches!(c,Data::Empty)) {return Err(format!("Columnas desconocidas en {name}"));}
        let mut map=Map::new();
        for (col, (_,field)) in cols.iter().enumerate() {
            let cell = cells.get(col).unwrap_or(&Data::Empty);
            let v = match *field {
                "mileage" | "partsCost" | "laborCost" | "charged" | "paid" => cell_as_number(cell, field)?,
                "year" => if matches!(cell,Data::Empty) || matches!(cell,Data::String(s) if s.is_empty()) {Value::String(String::new())} else {cell_as_number(cell,field)?},
                "demo" => match cell {Data::Bool(v)=>Value::Bool(*v),_=>return Err("La columna datos_demostracion debe contener VERDADERO/FALSO".into())},
                _=>Value::String(cell_as_string(cell).map_err(|e|format!("{name}, fila {}, columna {}: {e}",r+2,col+1))?),
            };
            map.insert((*field).to_string(),v);
        }
        out.push(Value::Object(map));
    }
    Ok(out)
}
fn manifest(excel:&mut Xlsx<Cursor<&[u8]>>)->Result<std::collections::HashMap<String,String>,String>{
    let rows=excel.worksheet_range("Manifest").map_err(xlsx_err)?;
    if rows.height()!=13 || rows.width()!=2 {return Err("Manifest incompleto".into());}
    let mut lines=rows.rows();
    let h=lines.next().ok_or("Manifest sin encabezado")?;
    if h[0].get_string()!=Some("clave") || h[1].get_string()!=Some("valor") {return Err("Manifest inválido".into());}
    let mut out=std::collections::HashMap::new();
    for row in lines {let key=cell_as_string(&row[0])?;let value=cell_as_string(&row[1])?;if out.insert(key,value).is_some(){return Err("Manifest con claves repetidas".into());}}
    Ok(out)
}
pub(crate) fn from_xlsx(bytes:&[u8])->Result<AppData,String>{
    if bytes.len()>25_000_000 {return Err("Excel demasiado grande (máximo 25 MB)".into());}
    let mut wb:Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(xlsx_err)?;
    let required=["Manifest","Clientes","Vehiculos","Trabajos","Configuracion"];
    let names=wb.sheet_names();
    if names.len()!=required.len() || required.iter().any(|s|!names.iter().any(|n|n==s)) {return Err("Hojas XLSX incompatibles con respaldo AutoTrace".into());}
    let meta=manifest(&mut wb)?;
    if meta.get("format").map(String::as_str)!=Some("autotrace-backup-xlsx") || meta.get("schema_version").map(String::as_str)!=Some("1") || meta.get("export_kind").map(String::as_str)!=Some("full") {return Err("Formato o versión de respaldo no admitidos".into());}
    let clients=read_rows(&mut wb,"Clientes",CLIENTS,15_000)?;
    let vehicles=read_rows(&mut wb,"Vehiculos",VEHICLES,30_000)?;
    let orders=read_rows(&mut wb,"Trabajos",ORDERS,100_000)?;
    let config=read_rows(&mut wb,"Configuracion",SETTINGS,1)?;
    if config.len()!=1 {return Err("La configuración debe tener una fila".into());}
    for (k,len) in [("count_clients",clients.len()),("count_vehicles",vehicles.len()),("count_orders",orders.len())]{if meta.get(k)!=Some(&len.to_string()){return Err(format!("Cantidad de registros alterada: {k}"));}}
    let data:AppData = serde_json::from_value(serde_json::json!({"clients":clients,"vehicles":vehicles,"orders":orders,"settings":config[0]})).map_err(xlsx_err)?;
    validate(&data)?;
    if meta.get("canonical_sha256")!=Some(&checksum(&data)?) {return Err("El Excel fue modificado o se dañó; checksum inválido".into());}
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xlsx_roundtrip_empty_model(){
        let data=AppData::default();
        let bytes=to_xlsx(&data).unwrap();
        assert!(bytes.starts_with(b"PK"));
        let restored=from_xlsx(&bytes).unwrap();
        assert_eq!(canonical_bytes(&data).unwrap(),canonical_bytes(&restored).unwrap());
    }
    #[test]
    fn xlsx_roundtrip_client_and_vehicle_with_text_ids(){
        let raw=serde_json::json!({"clients":[{"id":"00001","name":"Mónica Ñ","phone":"001234","email":"","notes":"=1+1","createdAt":"2026-10-21"}],"vehicles":[{"id":"v1","clientId":"00001","plate":"AF012BC","make":"Peugeot","model":"208","year":2023,"mileage":123.5,"vin":"","color":"Azul","nextServiceDate":"","notes":"","createdAt":"2026-10-21"}],"orders":[{"id":"o1","vehicleId":"v1","title":"Servicio","category":"Diagnóstico","status":"done","openedAt":"2026-10-21","completedAt":"","mileage":123.5,"description":"","partsCost":0.29,"laborCost":1.2,"charged":2.5,"paid":1.0,"notes":"privado","createdAt":"2026-10-21"}],"settings":{"garageName":"Taller Central","currency":"ARS","demo":false}});
        let data:AppData=serde_json::from_value(raw).unwrap();
        let bytes=to_xlsx(&data).unwrap();
        let restored=from_xlsx(&bytes).unwrap();
        assert_eq!(canonical_bytes(&data).unwrap(),canonical_bytes(&restored).unwrap());
    }
    #[test]
    fn rejects_unknown_workbook_and_corruption(){
        assert!(from_xlsx(b"invalid").is_err());
        let bytes=to_xlsx(&AppData::default()).unwrap();
        assert!(from_xlsx(&bytes[..bytes.len()/2]).is_err());
    }
}
