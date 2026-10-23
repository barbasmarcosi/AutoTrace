//! Public-facing, customer-scoped Excel report. NEVER include workshop costs,
//! margins, private notes, internal identifiers, VIN, or other customers.
use crate::store::{validate, AppData};
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use serde_json::Value;
use std::collections::HashSet;

fn field<'a>(x:&'a Value,key:&str)->&'a str{x.get(key).and_then(Value::as_str).unwrap_or("")}
fn fail(e:impl std::fmt::Display)->String{format!("No se pudo generar el informe: {e}")}
fn headers(ws:&mut Worksheet,cols:&[&str])->Result<(),String>{
    let style=Format::new().set_bold().set_background_color("#E5F0F6");
    for(i,title)in cols.iter().enumerate(){
        ws.write_string_with_format(0,i as u16,*title,&style).map_err(fail)?;
        ws.set_column_width(i as u16,if title.contains("Descripción"){48}else{23}).map_err(fail)?;
    }
    Ok(())
}
fn write_number(ws:&mut Worksheet,row:u32,col:u16,value:f64)->Result<(),String>{
    if !value.is_finite()||value<0.0{return Err("Importe o cantidad inválidos".into());}
    ws.write_number(row,col,value).map_err(fail)?;
    Ok(())
}
fn numeric(item:&Value,key:&str)->Result<f64,String>{
    item.get(key).and_then(Value::as_f64).ok_or_else(||format!("Campo inválido: {key}"))
}
fn classification(order:&Value)->&'static str{
    if field(order,"documentKind")=="quote"{"Presupuesto"}else{"Trabajo"}
}
/// Quotes are informative estimates, not accounts receivable. They must NOT
/// contribute balances or payments to the customer-facing workbook.
pub(crate) fn to_customer_xlsx(data:&AppData,customer_id:&str)->Result<Vec<u8>,String>{
    validate(data)?;
    let root=serde_json::to_value(data).map_err(fail)?;
    let clients=root["clients"].as_array().ok_or("Clientes inválidos")?;
    let client=clients.iter().find(|c|field(c,"id")==customer_id).ok_or("Cliente inexistente")?;
    let vehicles:Vec<&Value>=root["vehicles"].as_array().ok_or("Vehículos inválidos")?.iter()
        .filter(|v|field(v,"clientId")==customer_id).collect();
    let vehicle_ids:HashSet<&str>=vehicles.iter().map(|v|field(v,"id")).collect();
    let mut orders:Vec<&Value>=root["orders"].as_array().ok_or("Trabajos inválidos")?.iter()
        .filter(|o|vehicle_ids.contains(field(o,"vehicleId"))).collect();
    orders.sort_by(|a,b|field(b,"openedAt").cmp(field(a,"openedAt")));

    let mut book=Workbook::new();
    let sheet=book.add_worksheet();
    sheet.set_name("Resumen").map_err(fail)?;
    headers(sheet,&["Dato","Valor"])?;
    for (index,(label,value)) in [
        ("Taller",field(&root["settings"],"garageName")),
        ("Cliente",field(client,"name")),
        ("Teléfono",field(client,"phone")),
        ("Correo",field(client,"email")),
    ].iter().enumerate(){
        sheet.write_string((index+1)as u32,0,*label).map_err(fail)?;
        sheet.write_string((index+1)as u32,1,*value).map_err(fail)?;
    }
    sheet.write_string(6,0,"Informe informativo. No es una factura ni un respaldo.").map_err(fail)?;

    let sheet=book.add_worksheet();
    sheet.set_name("Vehículos").map_err(fail)?;
    headers(sheet,&["Patente","Vehículo","Año","Kilometraje","Próximo servicio"])?;
    for(index,vehicle)in vehicles.iter().enumerate(){
        let row=(index+1)as u32;
        sheet.write_string(row,0,field(vehicle,"plate")).map_err(fail)?;
        sheet.write_string(row,1,format!("{} {}",field(vehicle,"make"),field(vehicle,"model")).trim()).map_err(fail)?;
        sheet.write_string(row,2,vehicle["year"].as_i64().map(|n|n.to_string()).unwrap_or_default()).map_err(fail)?;
        write_number(sheet,row,3,numeric(vehicle,"mileage")?)?;
        sheet.write_string(row,4,field(vehicle,"nextServiceDate")).map_err(fail)?;
    }

    let sheet=book.add_worksheet();
    sheet.set_name("Servicios").map_err(fail)?;
    headers(sheet,&["Fecha","Patente","Documento","Trabajo","Estado","Descripción",
                    "Importe presupuestado/acordado","Abonado","Saldo"])?;
    for(index,order)in orders.iter().enumerate(){
        let row=(index+1)as u32;
        let plate=vehicles.iter().find(|v|field(v,"id")==field(order,"vehicleId"))
            .map(|v|field(v,"plate")).unwrap_or("");
        let is_quote=field(order,"documentKind")=="quote";
        for(col,value)in [
            field(order,"openedAt"),plate,classification(order),field(order,"title"),
            field(order,"status"),field(order,"description")
        ].iter().enumerate(){
            sheet.write_string(row,col as u16,*value).map_err(fail)?;
        }
        write_number(sheet,row,6,numeric(order,"charged")?)?;
        if !is_quote{
            let paid=numeric(order,"paid")?;
            write_number(sheet,row,7,paid)?;
            write_number(sheet,row,8,(numeric(order,"charged")?-paid).max(0.0))?;
        }
    }

    let sheet=book.add_worksheet();
    sheet.set_name("Detalle de trabajos").map_err(fail)?;
    headers(sheet,&["Fecha","Patente","Documento","Trabajo","Tipo de renglón",
                    "Descripción","Cantidad","Precio unitario","Subtotal"])?;
    let mut row=1u32;
    for order in &orders{
        let plate=vehicles.iter().find(|v|field(v,"id")==field(order,"vehicleId"))
            .map(|v|field(v,"plate")).unwrap_or("");
        if let Some(items)=order.get("items").and_then(Value::as_array){
            for line in items {
                let kind=match field(line,"kind"){"part"=>"Pieza / insumo","labor"=>"Mano de obra",_=>"Otro"};
                for(col,v)in [
                    field(order,"openedAt"),plate,classification(order),field(order,"title"),
                    kind,field(line,"description")
                ].iter().enumerate(){
                    sheet.write_string(row,col as u16,*v).map_err(fail)?;
                }
                let quantity=numeric(line,"quantity")?;
                let price=numeric(line,"unitPrice")?;
                write_number(sheet,row,6,quantity)?;
                write_number(sheet,row,7,price)?;
                write_number(sheet,row,8,(quantity*price*100.0).round()/100.0)?;
                row+=1;
            }
        }
    }
    book.save_to_buffer().map_err(fail)
}

#[cfg(test)]
mod tests{
    use super::*;
    use calamine::{Reader,Xlsx};
    use std::io::Cursor;

    #[test]
    fn report_is_itemized_scoped_and_quotes_are_not_collectable(){
        let raw=serde_json::json!({
          "clients":[
            {"id":"1","name":"Ana","phone":"0123","email":"","notes":"SECRETO_CLIENTE","createdAt":""},
            {"id":"2","name":"Omar","phone":"","email":"","notes":"","createdAt":""}],
          "vehicles":[
            {"id":"v1","clientId":"1","plate":"AF012BC","make":"Ford","model":"Ka","year":2018,"mileage":60000,"vin":"VIN_PRIVADO","color":"","nextServiceDate":"","notes":"NOTA_PRIVADA","createdAt":""},
            {"id":"v2","clientId":"2","plate":"AB123CD","make":"Fiat","model":"Uno","year":"","mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[
            {"id":"o1","vehicleId":"v1","title":"Revisión","category":"Mantenimiento","status":"done","openedAt":"2026-10-21","completedAt":"","mileage":60000,"description":"Bujías",
             "partsCost":811.5,"laborCost":921.0,"charged":3000,"paid":1000,"notes":"NOTA_TALLER","createdAt":"",
             "items":[{"id":"a","kind":"part","description":"FILTRO DE POLEN","quantity":2,"unitPrice":1000},
                      {"id":"b","kind":"labor","description":"CONTROL DE BUJÍAS","quantity":1,"unitPrice":1000}]},
            {"id":"o3","vehicleId":"v1","title":"Estimación","category":"Mecánica","status":"received","openedAt":"2026-10-20","completedAt":"","mileage":60000,"description":"Diagnóstico",
             "partsCost":0,"laborCost":0,"charged":4400,"paid":0,"notes":"","createdAt":"","documentKind":"quote",
             "items":[{"id":"a","kind":"other","description":"PRESUPUESTO DE PRUEBA","quantity":1,"unitPrice":4400}]},
            {"id":"o2","vehicleId":"v2","title":"SERVICIO_AJENO","category":"Mecánica","status":"done","openedAt":"","completedAt":"","mileage":0,"description":"",
             "partsCost":0,"laborCost":0,"charged":1,"paid":0,"notes":"","createdAt":"",
             "items":[{"id":"x","kind":"part","description":"RÉNGON_AJENO","quantity":1,"unitPrice":1}]}],
          "settings":{"garageName":"Taller Central","currency":"ARS","demo":false}
        });
        let data:AppData=serde_json::from_value(raw).unwrap();
        let bytes=to_customer_xlsx(&data,"1").unwrap();
        let mut book:Xlsx<_>=Xlsx::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(book.sheet_names(),vec!["Resumen","Vehículos","Servicios","Detalle de trabajos"]);
        let services=book.worksheet_range("Servicios").unwrap();
        assert_eq!(services.height(),3);
        assert_eq!(services.get((1,8)).unwrap().to_string(),"2000");
        // Quotes carry their estimated amount but intentionally no paid/due values.
        assert_eq!(services.get((2,2)).unwrap().to_string(),"Presupuesto");
        assert!(services.get((2,7)).is_none_or(|cell|matches!(cell,calamine::Data::Empty)));
        assert!(services.get((2,8)).is_none_or(|cell|matches!(cell,calamine::Data::Empty)));
        let detail=book.worksheet_range("Detalle de trabajos").unwrap();
        assert_eq!(detail.height(),4);
        assert_eq!(detail.get((1,5)).unwrap().to_string(),"FILTRO DE POLEN");
        assert_eq!(detail.get((1,8)).unwrap().to_string(),"2000");
        let mut visible=String::new();
        for name in book.sheet_names(){
            for cells in book.worksheet_range(&name).unwrap().rows(){
                for cell in cells{visible.push_str(&cell.to_string());visible.push('|');}
            }
        }
        for secret in ["SECRETO_CLIENTE","NOTA_PRIVADA","NOTA_TALLER","VIN_PRIVADO",
                       "SERVICIO_AJENO","RÉNGON_AJENO","AB123CD","811.5","921","partsCost","laborCost"]{
            assert!(!visible.contains(secret),"se filtró {secret}");
        }
        assert!(to_customer_xlsx(&data,"unknown").is_err());
        assert!(crate::excel::from_xlsx(&bytes).is_err(),"Informe no es respaldo importable");
    }
}
