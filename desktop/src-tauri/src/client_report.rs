//! XLSX client-facing reports; never a substitute for full private backups.
use crate::store::{validate,AppData};
use rust_xlsxwriter::{Format,Workbook};
use serde_json::Value;

fn field<'a>(x:&'a Value,key:&str)->&'a str{x.get(key).and_then(Value::as_str).unwrap_or("")}
fn fail(e:impl std::fmt::Display)->String{format!("No se pudo generar el informe: {e}")}
fn headers(ws:&mut rust_xlsxwriter::Worksheet, cols:&[&str])->Result<(),String>{
    let fmt=Format::new().set_bold().set_background_color("#E5F0F6");
    for(i,c)in cols.iter().enumerate(){ws.write_string_with_format(0,i as u16,*c,&fmt).map_err(fail)?;
        ws.set_column_width(i as u16,if *c=="Descripción"{46}else{23}).map_err(fail)?;}Ok(())
}
fn write_amount(ws:&mut rust_xlsxwriter::Worksheet,row:u32,col:u16,v:&Value,key:&str)->Result<(),String>{
    let n=v.get(key).and_then(Value::as_f64).ok_or_else(||format!("Importe inválido: {key}"))?;
    ws.write_number(row,col,n).map_err(fail)?;Ok(())
}
pub(crate) fn to_customer_xlsx(data:&AppData,client_id:&str)->Result<Vec<u8>,String>{
    validate(data)?;
    let v=serde_json::to_value(data).map_err(fail)?;
    let c=v["clients"].as_array().ok_or("Clientes inválidos")?.iter()
        .find(|x|field(x,"id")==client_id).ok_or("Cliente inexistente")?;
    let vs:Vec<&Value>=v["vehicles"].as_array().ok_or("Vehículos inválidos")?.iter()
        .filter(|x|field(x,"clientId")==client_id).collect();
    let ids:std::collections::HashSet<&str>=vs.iter().map(|x|field(x,"id")).collect();
    let mut os:Vec<&Value>=v["orders"].as_array().ok_or("Trabajos inválidos")?.iter()
        .filter(|x|ids.contains(field(x,"vehicleId"))).collect();
    os.sort_by(|a,b|field(b,"openedAt").cmp(field(a,"openedAt")));
    let mut wb=Workbook::new();
    let ws=wb.add_worksheet();ws.set_name("Resumen").map_err(fail)?;
    headers(ws,&["Dato","Valor"])?;
    for(i,(key,val))in [("Taller",field(&v["settings"],"garageName")),("Cliente",field(c,"name")),
        ("Teléfono",field(c,"phone")),("Correo",field(c,"email"))].iter().enumerate(){
        ws.write_string((i+1)as u32,0,*key).map_err(fail)?;
        ws.write_string((i+1)as u32,1,*val).map_err(fail)?;
    }
    ws.write_string(6,0,"Documento informativo. No es una factura.").map_err(fail)?;
    let ws=wb.add_worksheet();ws.set_name("Vehículos").map_err(fail)?;
    headers(ws,&["Patente","Vehículo","Año","Kilometraje","Próximo servicio"])?;
    for(i,item)in vs.iter().enumerate(){
        let row=(i+1)as u32;
        ws.write_string(row,0,field(item,"plate")).map_err(fail)?;
        ws.write_string(row,1,format!("{} {}",field(item,"make"),field(item,"model")).trim()).map_err(fail)?;
        ws.write_string(row,2,item["year"].as_i64().map(|x|x.to_string()).unwrap_or_default()).map_err(fail)?;
        write_amount(ws,row,3,item,"mileage")?;
        ws.write_string(row,4,field(item,"nextServiceDate")).map_err(fail)?;
    }
    let ws=wb.add_worksheet();ws.set_name("Servicios").map_err(fail)?;
    headers(ws,&["Fecha","Patente","Trabajo","Categoría","Estado","Descripción","Importe acordado","Pagado","Saldo"])?;
    for(i,item)in os.iter().enumerate(){
        let row=(i+1)as u32;
        let plate=vs.iter().find(|x|field(x,"id")==field(item,"vehicleId")).map(|x|field(x,"plate")).unwrap_or("");
        for(j,s)in [field(item,"openedAt"),plate,field(item,"title"),field(item,"category"),field(item,"status"),field(item,"description")].iter().enumerate(){
            ws.write_string(row,j as u16,*s).map_err(fail)?;
        }
        write_amount(ws,row,6,item,"charged")?;write_amount(ws,row,7,item,"paid")?;
        let due=(item["charged"].as_f64().unwrap_or(0.)-item["paid"].as_f64().unwrap_or(0.)).max(0.);
        ws.write_number(row,8,(due*100.).round()/100.).map_err(fail)?;
    }
    wb.save_to_buffer().map_err(fail)
}
#[cfg(test)]
mod tests{
    use super::*;
    use calamine::{Reader,Xlsx};
    use std::io::Cursor;
    #[test]
    fn client_report_does_not_leak_internal_or_other_customer_records(){
        let raw=serde_json::json!({
          "clients":[{"id":"1","name":"Ana","phone":"0123","email":"","notes":"SECRETO_CLIENTE","createdAt":""},{"id":"2","name":"Omar","phone":"","email":"","notes":"","createdAt":""}],
          "vehicles":[{"id":"v1","clientId":"1","plate":"AF012BC","make":"Ford","model":"Ka","year":2018,"mileage":60000,"vin":"VIN_PRIVADO","color":"","nextServiceDate":"","notes":"NOTA_PRIVADA","createdAt":""},{"id":"v2","clientId":"2","plate":"AB123CD","make":"Fiat","model":"Uno","year":"","mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[{"id":"o1","vehicleId":"v1","title":"Revisión","category":"Mantenimiento","status":"done","openedAt":"2026-10-21","completedAt":"","mileage":60000,"description":"Bujías","partsCost":811.5,"laborCost":921.0,"charged":3000,"paid":1000,"notes":"NOTA_TALLER","createdAt":""},{"id":"o2","vehicleId":"v2","title":"SERVICIO_AJENO","category":"Mecánica","status":"done","openedAt":"","completedAt":"","mileage":0,"description":"","partsCost":0,"laborCost":0,"charged":1,"paid":0,"notes":"","createdAt":""}],
          "settings":{"garageName":"Taller Central","currency":"ARS","demo":false}
        });
        let data:AppData=serde_json::from_value(raw).unwrap();
        let bytes=to_customer_xlsx(&data,"1").unwrap();
        let mut wb:Xlsx<_>=Xlsx::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(wb.sheet_names(),vec!["Resumen","Vehículos","Servicios"]);
        let services=wb.worksheet_range("Servicios").unwrap();
        assert_eq!(services.height(),2);
        assert_eq!(services.get((1,8)).unwrap().to_string(),"2000");
        let mut all=String::new();
        for name in wb.sheet_names(){for row in wb.worksheet_range(&name).unwrap().rows(){for cell in row{all.push_str(&cell.to_string());all.push('|');}}}
        for secret in ["SECRETO_CLIENTE","NOTA_PRIVADA","NOTA_TALLER","VIN_PRIVADO","SERVICIO_AJENO","811.5","921","AB123CD","partsCost","laborCost"]{assert!(!all.contains(secret),"filtró {secret}");}
        assert!(to_customer_xlsx(&data,"invalid").is_err());
        assert!(crate::excel::from_xlsx(&bytes).is_err());
    }
}