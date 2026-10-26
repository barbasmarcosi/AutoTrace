use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap,HashSet}, fs, path::PathBuf, time::Duration};
use tauri::Manager;

const SCHEMA: &str = include_str!("../migrations/001_init.sql");
const MAX_RECORDS: (usize, usize, usize) = (15_000, 30_000, 100_000);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Client {
    id: String,
    name: String,
    phone: String,
    email: String,
    notes: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Vehicle {
    id: String,
    client_id: String,
    plate: String,
    make: String,
    model: String,
    year: serde_json::Value, // PWA: año numérico o "" para desconocido.
    mileage: f64,
    vin: String,
    color: String,
    next_service_date: String,
    notes: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OrderItem {
    id: String,
    kind: String,
    description: String,
    quantity: f64,
    unit_price: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Payment {
    id: String,
    date: String,
    amount: f64,
    method: String,
    account_id: String,
    reference: String,
    #[serde(default,skip_serializing_if="String::is_empty")]
    voided_at: String,
    #[serde(default,skip_serializing_if="String::is_empty")]
    void_reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PaymentAccount {
    id: String,
    name: String,
    kind: String,
    alias: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Order {
    id: String,
    vehicle_id: String,
    title: String,
    category: String,
    status: String,
    opened_at: String,
    completed_at: String,
    mileage: f64,
    description: String,
    parts_cost: f64,
    labor_cost: f64,
    charged: f64,
    paid: f64,
    notes: String,
    created_at: String,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    items: Vec<OrderItem>,
    #[serde(default,skip_serializing_if="String::is_empty")]
    document_kind: String,
    #[serde(default,skip_serializing_if="String::is_empty")]
    document_number: String,
    #[serde(default,skip_serializing_if="String::is_empty")]
    source_fingerprint: String,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    payments: Vec<Payment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    garage_name: String,
    currency: String,
    demo: bool,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    accounts: Vec<PaymentAccount>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { garage_name: "Mi taller".into(), currency: "ARS".into(), demo: false, accounts: vec![] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppData {
    pub(crate) clients: Vec<Client>,
    pub(crate) vehicles: Vec<Vehicle>,
    pub(crate) orders: Vec<Order>,
    settings: Settings,
}
impl AppData {
    pub(crate) fn clients_count(&self)->usize{self.clients.len()}
    pub(crate) fn vehicles_count(&self)->usize{self.vehicles.len()}
    pub(crate) fn orders_count(&self)->usize{self.orders.len()}
}
impl Default for AppData {
    fn default() -> Self {
        Self { clients: vec![], vehicles: vec![], orders: vec![], settings: Settings::default() }
    }
}

#[derive(Serialize)]
pub struct Snapshot {
    pub(crate) data: AppData,
    pub(crate) revision: u64,
}

#[derive(Serialize)]
pub struct SaveResult {
    revision: u64,
}

fn sqlite_error(e: impl std::fmt::Display) -> String {
    format!("No se pudo acceder a la base de datos local: {e}")
}

fn open(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(sqlite_error)?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(sqlite_error)?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(sqlite_error)?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(sqlite_error)?;
    if version > 1 { return Err("La base de datos es de una versión más reciente. No se modificó.".into()); }
    if version == 0 {
        // A new DB has no schema. Never retrofit an unrelated or damaged
        // pre-existing SQLite file by silently creating AutoTrace tables.
        let existing_objects:i64=conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
            [],|r|r.get(0)).map_err(sqlite_error)?;
        if existing_objects!=0 {
            return Err("La base existente contiene objetos sin versión AutoTrace. No se modificó.".into());
        }
        conn.execute_batch(&format!("BEGIN IMMEDIATE;\n{SCHEMA}\nCOMMIT;"))
            .map_err(sqlite_error)?;
    }
    Ok(conn)
}

pub(crate) fn open_app(app: &tauri::AppHandle) -> Result<Connection, String> {
    let dir: PathBuf = app.path().app_local_data_dir().map_err(sqlite_error)?;
    fs::create_dir_all(&dir).map_err(sqlite_error)?;
    open(&dir.join("autotrace.sqlite3"))
}

fn valid_text(value: &str, max: usize, field: &str, required: bool) -> Result<(), String> {
    if (required && value.trim().is_empty()) || value.chars().count() > max {
        return Err(format!("Campo inválido: {field}"));
    }
    Ok(())
}
fn valid_date(value: &str) -> bool {
    if value.is_empty() { return true; }
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' { return false; }
    let Some(year) = value.get(0..4).and_then(|s| s.parse::<u32>().ok()) else { return false; };
    let Some(month) = value.get(5..7).and_then(|s| s.parse::<u32>().ok()) else { return false; };
    let Some(day) = value.get(8..10).and_then(|s| s.parse::<u32>().ok()) else { return false; };
    if !(1..=12).contains(&month) || year < 1900 { return false; }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month { 2 => if leap { 29 } else { 28 }, 4 | 6 | 9 | 11 => 30, _ => 31 };
    (1..=days).contains(&day)
}
fn check_date(value: &str, field: &str) -> Result<(), String> {
    if valid_date(value) { Ok(()) } else { Err(format!("Fecha inválida en {field}")) }
}
fn natural(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && (0.0..=1e12).contains(&value) { Ok(()) }
    else { Err(format!("Número inválido en {field}")) }
}
fn cents(value: f64, field: &str) -> Result<i64, String> {
    natural(value, field)?;
    let rounded = (value * 100.0).round();
    if ((value * 100.0) - rounded).abs() > 0.00001 {
        return Err(format!("{field} tiene más de dos decimales"));
    }
    Ok(rounded as i64)
}
fn normalized_plate(plate: &str) -> String {
    plate.chars().filter(|c| c.is_ascii_alphanumeric()).flat_map(|c| c.to_uppercase()).collect()
}

pub(crate) fn validate(data: &AppData) -> Result<(), String> {
    if data.clients.len() > MAX_RECORDS.0 || data.vehicles.len() > MAX_RECORDS.1 || data.orders.len() > MAX_RECORDS.2 {
        return Err("Se superó el límite admitido de registros".into());
    }
    valid_text(&data.settings.garage_name, 100, "taller", true)?;
    if !matches!(data.settings.currency.as_str(), "ARS" | "USD" | "EUR") {
        return Err("Moneda inválida".into());
    }
    if data.settings.accounts.len()>100 {return Err("Demasiadas cuentas receptoras".into());}
    let mut account_ids=HashSet::new();
    for a in &data.settings.accounts {
        valid_text(&a.id,100,"cuenta.id",true)?;
        valid_text(&a.name,120,"cuenta.nombre",true)?;
        valid_text(&a.alias,120,"cuenta.alias",false)?;
        if !matches!(a.kind.as_str(),"bank"|"wallet"|"other") || !account_ids.insert(a.id.as_str()) {
            return Err("Cuenta receptora inválida o duplicada".into());
        }
    }
    let mut clients = HashSet::new();
    for c in &data.clients {
        valid_text(&c.id, 100, "cliente.id", true)?;
        valid_text(&c.name, 120, "cliente.nombre", true)?;
        valid_text(&c.phone, 120, "cliente.telefono", false)?;
        valid_text(&c.email, 120, "cliente.email", false)?;
        valid_text(&c.notes, 3000, "cliente.notas", false)?;
        check_date(&c.created_at, "cliente.creadoEl")?;
        if !clients.insert(c.id.as_str()) { return Err("Cliente duplicado".into()); }
    }
    let mut vehicles = HashSet::new();
    let mut plates = HashSet::new();
    for v in &data.vehicles {
        valid_text(&v.id, 100, "vehiculo.id", true)?;
        valid_text(&v.make, 120, "vehiculo.marca", true)?;
        valid_text(&v.model, 120, "vehiculo.modelo", true)?;
        valid_text(&v.vin, 40, "vehiculo.vin", false)?;
        valid_text(&v.color, 120, "vehiculo.color", false)?;
        valid_text(&v.notes, 3000, "vehiculo.notas", false)?;
        check_date(&v.next_service_date, "vehiculo.proximoServicio")?;
        check_date(&v.created_at, "vehiculo.creadoEl")?;
        natural(v.mileage, "vehiculo.kilometraje")?;
        if !clients.contains(v.client_id.as_str()) { return Err("Vehículo sin cliente".into()); }
        let plate = normalized_plate(&v.plate);
        if plate != v.plate || !(5..=12).contains(&plate.len()) { return Err("Patente inválida".into()); }
        if !vehicles.insert(v.id.as_str()) || !plates.insert(plate) { return Err("Vehículo o patente duplicados".into()); }
        if v.year != serde_json::Value::String(String::new()) {
            let Some(y) = v.year.as_i64() else { return Err("Año de vehículo inválido".into()); };
            if !(1900..=2200).contains(&y) { return Err("Año de vehículo fuera de rango".into()); }
        }
    }
    let mut orders = HashSet::new();
    // Payment IDs identify ledger events across the entire workshop, not just
    // within a work order. A reused ID must never appear as two cash receipts.
    let mut payment_ids = HashSet::new();
    // A single imported workbook fingerprint cannot create two orders and
    // consequently duplicate both revenue and receivables.
    let mut source_fingerprints = HashSet::new();
    for o in &data.orders {
        valid_text(&o.id, 100, "trabajo.id", true)?;
        valid_text(&o.title, 150, "trabajo.titulo", true)?;
        valid_text(&o.category, 120, "trabajo.categoria", false)?;
        valid_text(&o.description, 3000, "trabajo.descripcion", false)?;
        valid_text(&o.notes, 3000, "trabajo.notas", false)?;
        check_date(&o.opened_at, "trabajo.apertura")?;
        check_date(&o.completed_at, "trabajo.cierre")?;
        check_date(&o.created_at, "trabajo.creadoEl")?;
        natural(o.mileage, "trabajo.kilometraje")?;
        if !vehicles.contains(o.vehicle_id.as_str()) { return Err("Trabajo sin vehículo".into()); }
        if !matches!(o.status.as_str(), "received" | "progress" | "waiting" | "done" | "delivered") {
            return Err("Estado de trabajo inválido".into());
        }
        if cents(o.paid, "trabajo.pagado")? > cents(o.charged, "trabajo.importe")? {
            return Err("Pago mayor al importe del trabajo".into());
        }
        cents(o.parts_cost, "trabajo.repuestos")?;
        cents(o.labor_cost, "trabajo.manoDeObra")?;
        if !o.document_kind.is_empty() && !matches!(o.document_kind.as_str(),"quote"|"work"){
            return Err("Clase de documento inválida".into());
        }
        valid_text(&o.document_number,80,"documento.numero",false)?;
        if !o.source_fingerprint.is_empty() && (o.source_fingerprint.len()!=64
           || !o.source_fingerprint.chars().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase())){
            return Err("Huella digital del archivo inválida".into());
        }
        if !o.source_fingerprint.is_empty() &&
            !source_fingerprints.insert(o.source_fingerprint.as_str()) {
            return Err("Documento de origen duplicado entre trabajos; se bloqueó la contabilización doble".into());
        }
        if o.payments.len()>200 {return Err("Demasiados cobros para un trabajo".into());}
        let mut detailed_cents=0_i64;
        for p in &o.payments {
            valid_text(&p.id,100,"cobro.id",true)?;
            if !payment_ids.insert(p.id.as_str()) {return Err("Cobro duplicado".into());}
            if p.date.is_empty(){return Err("El cobro requiere fecha".into());}
            check_date(&p.date,"cobro.fecha")?;
            valid_text(&p.reference,160,"cobro.referencia",false)?;
            if p.voided_at.is_empty()!=p.void_reason.is_empty(){
                return Err("La anulación requiere fecha y motivo juntos".into());
            }
            if !p.voided_at.is_empty(){
                if !valid_date(&p.voided_at)||p.voided_at<p.date{
                    return Err("Fecha de anulación inválida o anterior al cobro".into());
                }
                valid_text(&p.void_reason,500,"motivo de anulación",true)?;
                if p.void_reason.trim().chars().count()<12{
                    return Err("El motivo de anulación requiere al menos 12 caracteres".into());
                }
            }
            if !matches!(p.method.as_str(),"cash"|"transfer"|"debit"|"credit"|"qr"|"other"){return Err("Medio de pago inválido".into());}
            let electronic=matches!(p.method.as_str(),"transfer"|"debit"|"credit"|"qr");
            if (electronic&&!account_ids.contains(p.account_id.as_str()))||(!electronic&&!p.account_id.is_empty()){
                return Err("Cuenta receptora incompatible con el medio de pago".into());
            }
            let value=cents(p.amount,"cobro.importe")?;
            if value<=0 {return Err("Un cobro debe ser positivo".into());}
            if p.voided_at.is_empty(){
                detailed_cents=detailed_cents.checked_add(value).ok_or("Acumulado de cobros excesivo")?;
            }
        }
        if detailed_cents>cents(o.paid,"trabajo.pagado")? {return Err("El detalle de cobros supera el total recibido".into());}
        if o.document_kind=="quote" && (o.paid>0.0||!o.payments.is_empty()){return Err("Los presupuestos no tienen cobros".into());}
        if o.items.len()>100{return Err("Demasiados renglones".into());}
        let mut lines=HashSet::new();
        for line in &o.items {
            valid_text(&line.id,100,"renglon.id",true)?;
            valid_text(&line.description,1500,"renglon.descripcion",true)?;
            if !matches!(line.kind.as_str(),"part"|"labor"|"other"){return Err("Tipo de renglón inválido".into());}
            natural(line.quantity,"renglon.cantidad")?;
            if line.quantity<=0.0||line.quantity>100000.0{return Err("Cantidad inválida".into());}
            cents(line.unit_price,"renglon.precio")?;
            if !lines.insert(line.id.as_str()){return Err("Renglón duplicado".into());}
            // Los insumos fraccionarios (p. ej. 1,25 litros) se liquidan redondeando
            // cada subtotal a centavos. El producto intermedio no tiene que ser
            // exactamente múltiplo de 0,01.
            natural(line.quantity*line.unit_price,"renglon.total")?;
        }
        if !orders.insert(o.id.as_str()) { return Err("Trabajo duplicado".into()); }
    }
    Ok(())
}

fn current_revision(conn: &Connection) -> Result<u64, String> {
    let val: i64 = conn.query_row("SELECT int_value FROM app_meta WHERE key='revision'", [], |r| r.get(0)).map_err(sqlite_error)?;
    u64::try_from(val).map_err(sqlite_error)
}

/// Reject persisted SQLite rows whose indexed columns disagree with the
/// authoritative JSON record. PRAGMA integrity_check and foreign_key_check
/// cannot detect these logical inconsistencies after external file edits.
pub(crate) fn read(conn: &Connection) -> Result<Snapshot, String> {
    let mut data = AppData::default();
    {
        let mut stmt=conn.prepare("SELECT id,name,record_json FROM clients ORDER BY rowid").map_err(sqlite_error)?;
        let rows=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))
            .map_err(sqlite_error)?;
        for row in rows {
            let (id,name,json)=row.map_err(sqlite_error)?;
            let client:Client=serde_json::from_str(&json).map_err(sqlite_error)?;
            if client.id!=id||client.name!=name {
                return Err("La base SQLite contiene datos de cliente inconsistentes. No se cargó.".into());
            }
            data.clients.push(client);
        }
    }
    {
        let mut stmt=conn.prepare("SELECT id,client_id,plate,record_json FROM vehicles ORDER BY rowid").map_err(sqlite_error)?;
        let rows=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))
            .map_err(sqlite_error)?;
        for row in rows {
            let (id,client_id,plate,json)=row.map_err(sqlite_error)?;
            let vehicle:Vehicle=serde_json::from_str(&json).map_err(sqlite_error)?;
            if vehicle.id!=id||vehicle.client_id!=client_id||vehicle.plate!=plate {
                return Err("La base SQLite contiene datos de vehículo inconsistentes. No se cargó.".into());
            }
            data.vehicles.push(vehicle);
        }
    }
    {
        let mut stmt=conn.prepare(
            "SELECT id,vehicle_id,status,charged_cents,paid_cents,parts_cents,labor_cents,record_json FROM orders ORDER BY rowid"
        ).map_err(sqlite_error)?;
        let rows=stmt.query_map([],|r|Ok((
            r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,
            r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,
            r.get::<_,i64>(6)?,r.get::<_,String>(7)?
        ))).map_err(sqlite_error)?;
        for row in rows {
            let (id,vehicle_id,status,charged,paid,parts,labor,json)=row.map_err(sqlite_error)?;
            let order:Order=serde_json::from_str(&json).map_err(sqlite_error)?;
            if order.id!=id||order.vehicle_id!=vehicle_id||order.status!=status||
                cents(order.charged,"importe")?!=charged||
                cents(order.paid,"pagado")?!=paid||
                cents(order.parts_cost,"repuestos")?!=parts||
                cents(order.labor_cost,"mano de obra")?!=labor {
                return Err("La base SQLite contiene importes o datos de trabajo inconsistentes. No se cargó.".into());
            }
            data.orders.push(order);
        }
    }
    if let Some(raw)=conn.query_row("SELECT record_json FROM app_settings WHERE id=1",[],|r|r.get::<_,String>(0))
        .optional().map_err(sqlite_error)? {
        data.settings=serde_json::from_str(&raw).map_err(sqlite_error)?;
    }
    validate(&data)?;
    Ok(Snapshot{data,revision:current_revision(conn)?})
}

/// Load each table once under the same IMMEDIATE transaction. Avoid 50k
/// repeated UPSERT statements on a single-field edit of a large workshop.
fn stored_records(conn:&Connection,table:&str)->Result<HashMap<String,String>,String>{
    let sql=match table{
        "orders"=>"SELECT id,record_json FROM orders",
        "vehicles"=>"SELECT id,record_json FROM vehicles",
        "clients"=>"SELECT id,record_json FROM clients",
        _=>return Err("Entidad de persistencia no soportada".into()),
    };
    let mut stmt=conn.prepare(sql).map_err(sqlite_error)?;
    let rows=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))
        .map_err(sqlite_error)?;
    rows.collect::<rusqlite::Result<HashMap<_,_>>>().map_err(sqlite_error)
}
/// Remove records absent from the submitted snapshot. Deletion is ordered from
/// child to parent to preserve FK integrity, while existing IDs retain rowid.
fn remove_deleted(conn:&Connection,table:&str,stored:&HashMap<String,String>,expected:&HashSet<&str>)->Result<(),String>{
    let sql=match table{
        "orders"=>"DELETE FROM orders WHERE id=?1",
        "vehicles"=>"DELETE FROM vehicles WHERE id=?1",
        "clients"=>"DELETE FROM clients WHERE id=?1",
        _=>return Err("Entidad de persistencia no soportada".into()),
    };
    let mut delete=conn.prepare(sql).map_err(sqlite_error)?;
    for id in stored.keys(){
        if !expected.contains(id.as_str()){delete.execute([id]).map_err(sqlite_error)?;}
    }
    Ok(())
}

/// SQLite's unique plate index would reject valid swaps (A->B and B->A)
/// if we upserted one vehicle before another. Release changed existing plates
/// using short unique placeholders only inside the active transaction.
fn release_changed_vehicle_plates(conn:&Connection,data:&AppData)->Result<(),String>{
    let target:HashMap<&str,&str>=data.vehicles.iter().map(|v|(v.id.as_str(),v.plate.as_str())).collect();
    let rows={
        let mut stmt=conn.prepare("SELECT id,plate FROM vehicles").map_err(sqlite_error)?;
        let read=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))
            .map_err(sqlite_error)?;
        read.collect::<rusqlite::Result<Vec<_>>>().map_err(sqlite_error)?
    };
    let mut reserved:HashSet<String>=rows.iter().map(|(_,p)|p.clone()).collect();
    reserved.extend(data.vehicles.iter().map(|v|v.plate.clone()));
    for (id,old_plate) in rows{
        if target.get(id.as_str()).is_some_and(|wanted|*wanted!=old_plate.as_str()){
            let placeholder=loop{
                let id=uuid::Uuid::new_v4().simple().to_string();
                let token=format!("TMP{}",id[..8].to_ascii_uppercase());
                if reserved.insert(token.clone()){break token;}
            };
            conn.execute("UPDATE vehicles SET plate=?1 WHERE id=?2",params![placeholder,id])
                .map_err(sqlite_error)?;
        }
    }
    Ok(())
}

/// Transactional incremental persistence: avoid DELETE ALL / INSERT ALL on
/// each phone edit or payment. The complete submitted model is still validated,
/// and revision checking prevents concurrent overwrites from other windows.
pub(crate) fn write(conn: &mut Connection, data: &AppData, expected_revision: u64) -> Result<SaveResult, String> {
    validate(data)?;
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sqlite_error)?;
    let existing=current_revision(&tx)?;
    if existing!=expected_revision {
        return Err("Otra ventana modificó los registros. Cerrá y volvé a abrir antes de guardar.".into());
    }
    let client_ids:HashSet<&str>=data.clients.iter().map(|c|c.id.as_str()).collect();
    let vehicle_ids:HashSet<&str>=data.vehicles.iter().map(|v|v.id.as_str()).collect();
    let order_ids:HashSet<&str>=data.orders.iter().map(|o|o.id.as_str()).collect();
    // Snapshot IDs and JSON once per transaction before any plate placeholders
    // or ordered deletions. No stale cache survives between revisions.
    let stored_clients=stored_records(&tx,"clients")?;
    let stored_vehicles=stored_records(&tx,"vehicles")?;
    let stored_orders=stored_records(&tx,"orders")?;
    remove_deleted(&tx,"orders",&stored_orders,&order_ids)?;
    remove_deleted(&tx,"vehicles",&stored_vehicles,&vehicle_ids)?;
    remove_deleted(&tx,"clients",&stored_clients,&client_ids)?;
    {
        let mut stmt=tx.prepare(
            "INSERT INTO clients(id,name,record_json) VALUES (?1,?2,?3)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,record_json=excluded.record_json
             WHERE clients.record_json!=excluded.record_json OR clients.name!=excluded.name"
        ).map_err(sqlite_error)?;
        for c in &data.clients{
            let json=serde_json::to_string(c).map_err(sqlite_error)?;
            if stored_clients.get(&c.id).is_some_and(|saved|saved==&json){continue;}
            stmt.execute(params![c.id,c.name,json]).map_err(sqlite_error)?;
        }
    }
    release_changed_vehicle_plates(&tx,data)?;
    {
        let mut stmt=tx.prepare(
            "INSERT INTO vehicles(id,client_id,plate,record_json) VALUES (?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET
               client_id=excluded.client_id,plate=excluded.plate,record_json=excluded.record_json
             WHERE vehicles.record_json!=excluded.record_json OR
               vehicles.client_id!=excluded.client_id OR vehicles.plate!=excluded.plate"
        ).map_err(sqlite_error)?;
        for v in &data.vehicles{
            let json=serde_json::to_string(v).map_err(sqlite_error)?;
            if stored_vehicles.get(&v.id).is_some_and(|saved|saved==&json){continue;}
            stmt.execute(params![v.id,v.client_id,v.plate,json]).map_err(sqlite_error)?;
        }
    }
    {
        let mut stmt=tx.prepare(
            "INSERT INTO orders(id,vehicle_id,status,charged_cents,paid_cents,parts_cents,labor_cents,record_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET
               vehicle_id=excluded.vehicle_id,status=excluded.status,charged_cents=excluded.charged_cents,
               paid_cents=excluded.paid_cents,parts_cents=excluded.parts_cents,labor_cents=excluded.labor_cents,
               record_json=excluded.record_json
             WHERE orders.record_json!=excluded.record_json OR
               orders.vehicle_id!=excluded.vehicle_id OR orders.status!=excluded.status OR
               orders.charged_cents!=excluded.charged_cents OR orders.paid_cents!=excluded.paid_cents OR
               orders.parts_cents!=excluded.parts_cents OR orders.labor_cents!=excluded.labor_cents"
        ).map_err(sqlite_error)?;
        for o in &data.orders{
            let json=serde_json::to_string(o).map_err(sqlite_error)?;
            if stored_orders.get(&o.id).is_some_and(|saved|saved==&json){continue;}
            stmt.execute(params![
                o.id,o.vehicle_id,o.status,cents(o.charged,"importe")?,
                cents(o.paid,"pagado")?,cents(o.parts_cost,"repuestos")?,
                cents(o.labor_cost,"mano de obra")?,json
            ]).map_err(sqlite_error)?;
        }
    }
    tx.execute(
        "INSERT INTO app_settings(id,record_json) VALUES(1,?1)
         ON CONFLICT(id) DO UPDATE SET record_json=excluded.record_json
         WHERE app_settings.record_json!=excluded.record_json",
        [serde_json::to_string(&data.settings).map_err(sqlite_error)?]
    ).map_err(sqlite_error)?;
    let next=existing.checked_add(1).ok_or("Se agotó la revisión de datos")?;
    let sql_next=i64::try_from(next).map_err(|_|"Revisión fuera del rango de SQLite")?;
    tx.execute("UPDATE app_meta SET int_value=?1 WHERE key='revision'",[sql_next])
        .map_err(sqlite_error)?;
    tx.commit().map_err(sqlite_error)?;
    Ok(SaveResult{revision:next})
}

#[tauri::command]
pub fn load_state(app: tauri::AppHandle) -> Result<Snapshot, String> {
    read(&open_app(&app)?)
}

#[tauri::command]
pub fn save_state(app: tauri::AppHandle, data: AppData, expected_revision: u64) -> Result<SaveResult, String> {
    write(&mut open_app(&app)?, &data, expected_revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_v1_sqlite_survives_reopen_and_future_version_is_byte_identical(){
        let base=std::env::temp_dir().join(format!("autotrace-schema-upgrade-{}",uuid::Uuid::new_v4()));
        fs::create_dir(&base).unwrap();
        let file=base.join("autotrace.sqlite3");
        let mut db=open(&file).unwrap();
        let v1:AppData=serde_json::from_value(serde_json::json!({
            "clients":[{"id":"c1","name":"Cliente histórico","phone":"","email":"","notes":"","createdAt":""}],
            "vehicles":[{"id":"v1","clientId":"c1","plate":"AF123BC","make":"Ford","model":"Fiesta",
                         "year":2018,"mileage":12000,"vin":"","color":"","nextServiceDate":"",
                         "notes":"","createdAt":""}],
            "orders":[{"id":"o1","vehicleId":"v1","title":"Trabajo legado","category":"","status":"received",
                       "openedAt":"","completedAt":"","mileage":12000,"description":"",
                       "partsCost":23.45,"laborCost":12.0,"charged":40.0,"paid":10.0,
                       "notes":"","createdAt":""}],
            "settings":{"garageName":"Taller histórico","currency":"ARS","demo":false}
        })).unwrap();
        write(&mut db,&v1,0).unwrap();
        let saved=serde_json::to_value(&v1).unwrap();
        drop(db);
        let reopened=open(&file).unwrap();
        let snapshot=read(&reopened).unwrap();
        assert_eq!(snapshot.revision,1);
        assert_eq!(serde_json::to_value(snapshot.data).unwrap(),saved);
        drop(reopened);
        let future=Connection::open(&file).unwrap();
        future.execute_batch("PRAGMA user_version=2;").unwrap();
        drop(future);
        let before=fs::read(&file).unwrap();
        assert!(open(&file).is_err(),"a newer version must not be migrated backwards");
        assert_eq!(fs::read(&file).unwrap(),before,"rejected future schema must stay byte-for-byte intact");
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn unversioned_nonempty_sqlite_is_rejected_without_touching_any_bytes(){
        let base=std::env::temp_dir().join(format!("autotrace-foreign-schema-{}",uuid::Uuid::new_v4()));
        fs::create_dir(&base).unwrap();
        let file=base.join("autotrace.sqlite3");
        let foreign=Connection::open(&file).unwrap();
        foreign.execute_batch("CREATE TABLE unrelated(id INTEGER PRIMARY KEY, note TEXT); INSERT INTO unrelated(note) VALUES('synthetic');").unwrap();
        assert_eq!(foreign.pragma_query_value(None,"user_version",|r|r.get::<_,i64>(0)).unwrap(),0);
        drop(foreign);
        let before=fs::read(&file).unwrap();
        let err=open(&file).err().expect("version-less foreign data must fail closed");
        assert!(err.contains("sin versión AutoTrace"));
        assert_eq!(fs::read(&file).unwrap(),before,"foreign database must remain completely unchanged");
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn empty_database_starts_without_demo_and_preserves_revision() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        let initial = read(&db).unwrap();
        assert!(initial.data.clients.is_empty());
        assert!(!initial.data.settings.demo);
        assert_eq!(initial.revision, 0);
        let first = write(&mut db, &initial.data, initial.revision).unwrap();
        assert_eq!(first.revision, 1);
        assert!(write(&mut db, &initial.data, 0).is_err());
        assert_eq!(read(&db).unwrap().revision, 1);
    }
    #[test]
    fn rejects_bad_dates_and_amounts() {
        assert!(!valid_date("2026-02-31"));
        assert!(valid_date("2024-02-29"));
        assert_eq!(cents(0.29, "amount").unwrap(), 29);
        assert!(cents(-1.0, "amount").is_err());
        assert!(cents(1.234, "amount").is_err());
    }
    #[test]
    fn fractional_fluid_quantities_round_subtotals_to_cents(){
        let payload=serde_json::json!({
            "clients":[{"id":"c","name":"Cliente sintético","phone":"","email":"","notes":"","createdAt":""}],
            "vehicles":[{"id":"v","clientId":"c","plate":"AF012BC","make":"Auto","model":"Modelo","year":"",
                          "mileage":1,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
            "orders":[{"id":"o","vehicleId":"v","title":"Lubricante fraccionario","category":"Servicio",
                       "status":"received","openedAt":"2026-10-22","completedAt":"","mileage":1,
                       "description":"","partsCost":0,"laborCost":0,"charged":41.66,"paid":0,
                       "notes":"","createdAt":"","items":[{"id":"l","kind":"part","description":"Aceite",
                       "quantity":1.25,"unitPrice":33.33}]}],
            "settings":{"garageName":"Taller","currency":"ARS","demo":false}
        });
        let value:AppData=serde_json::from_value(payload).unwrap();
        assert!(validate(&value).is_ok());
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        write(&mut db,&value,0).unwrap();
        let restored=read(&db).unwrap().data;
        assert_eq!(serde_json::to_value(restored).unwrap()["orders"][0]["items"][0]["quantity"],1.25);
    }
    #[test]
    fn incremental_write_preserves_existing_rows_and_updates_only_changed_entities(){
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db.execute_batch(SCHEMA).unwrap();
        let fixture=serde_json::json!({
            "clients":[{"id":"c1","name":"Cliente Uno","phone":"223000","email":"","notes":"","createdAt":"2026-10-24"}],
            "vehicles":[{"id":"v1","clientId":"c1","plate":"AF123BC","make":"Toyota","model":"Corolla",
                         "year":2021,"mileage":2500,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":"2026-10-24"}],
            "orders":[{"id":"o1","vehicleId":"v1","title":"Diagnóstico","category":"Mecánica","status":"received",
                       "openedAt":"2026-10-24","completedAt":"","mileage":2500,"description":"","partsCost":100,
                       "laborCost":50,"charged":300,"paid":100,"notes":"","createdAt":"2026-10-24"}],
            "settings":{"garageName":"Taller de prueba","currency":"ARS","demo":false}
        });
        let mut data:AppData=serde_json::from_value(fixture).unwrap();
        assert_eq!(write(&mut db,&data,0).unwrap().revision,1);
        let original_row_ids:(i64,i64,i64)=(
            db.query_row("SELECT rowid FROM clients WHERE id='c1'",[],|r|r.get(0)).unwrap(),
            db.query_row("SELECT rowid FROM vehicles WHERE id='v1'",[],|r|r.get(0)).unwrap(),
            db.query_row("SELECT rowid FROM orders WHERE id='o1'",[],|r|r.get(0)).unwrap()
        );
        // An unchanged snapshot must neither DELETE nor UPDATE any business row.
        db.execute_batch("
            CREATE TRIGGER deny_client_delete BEFORE DELETE ON clients
            BEGIN SELECT RAISE(ABORT,'unexpected client delete'); END;
            CREATE TRIGGER deny_vehicle_delete BEFORE DELETE ON vehicles
            BEGIN SELECT RAISE(ABORT,'unexpected vehicle delete'); END;
            CREATE TRIGGER deny_order_delete BEFORE DELETE ON orders
            BEGIN SELECT RAISE(ABORT,'unexpected order delete'); END;
            CREATE TRIGGER deny_vehicle_update BEFORE UPDATE ON vehicles
            BEGIN SELECT RAISE(ABORT,'unexpected vehicle update'); END;
            CREATE TRIGGER deny_order_update BEFORE UPDATE ON orders
            BEGIN SELECT RAISE(ABORT,'unexpected order update'); END;
            CREATE TRIGGER deny_client_insert BEFORE INSERT ON clients
            BEGIN SELECT RAISE(ABORT,'unexpected client insert'); END;
            CREATE TRIGGER deny_vehicle_insert BEFORE INSERT ON vehicles
            BEGIN SELECT RAISE(ABORT,'unexpected vehicle insert'); END;
            CREATE TRIGGER deny_order_insert BEFORE INSERT ON orders
            BEGIN SELECT RAISE(ABORT,'unexpected order insert'); END;
        ").unwrap();
        assert_eq!(write(&mut db,&data,1).unwrap().revision,2);
        // Existing rows must be skipped before even attempting INSERT; an
        // ON CONFLICT WHERE false still fires SQLite's BEFORE INSERT trigger.
        db.execute_batch("DROP TRIGGER deny_client_insert; DROP TRIGGER deny_vehicle_insert;
                          DROP TRIGGER deny_order_insert;").unwrap();
        // Editing a client MUST NOT touch a vehicle or its order.
        data.clients[0].name="Cliente Renombrado".into();
        assert_eq!(write(&mut db,&data,2).unwrap().revision,3);
        let row_ids:(i64,i64,i64)=(
            db.query_row("SELECT rowid FROM clients WHERE id='c1'",[],|r|r.get(0)).unwrap(),
            db.query_row("SELECT rowid FROM vehicles WHERE id='v1'",[],|r|r.get(0)).unwrap(),
            db.query_row("SELECT rowid FROM orders WHERE id='o1'",[],|r|r.get(0)).unwrap()
        );
        assert_eq!(original_row_ids,row_ids);
        assert_eq!(read(&db).unwrap().data.clients[0].name,"Cliente Renombrado");
        assert!(write(&mut db,&data,2).is_err(),"stale optimistic revision must fail");
        assert_eq!(read(&db).unwrap().revision,3);
        // Targeted deletions still obey child-before-parent order under active FK checks.
        db.execute_batch("DROP TRIGGER deny_client_delete; DROP TRIGGER deny_vehicle_delete;
                          DROP TRIGGER deny_order_delete; DROP TRIGGER deny_vehicle_update;
                          DROP TRIGGER deny_order_update;").unwrap();
        data.orders.clear();data.vehicles.clear();data.clients.clear();
        assert_eq!(write(&mut db,&data,3).unwrap().revision,4);
        let empty=read(&db).unwrap().data;
        assert!(empty.clients.is_empty()&&empty.vehicles.is_empty()&&empty.orders.is_empty());
    }
    #[test]
    fn swapping_unique_vehicle_plates_is_atomic_and_does_not_break_foreign_keys(){
        let mut conn=Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        let doc=serde_json::json!({
            "clients":[{"id":"c","name":"Dueño","phone":"","email":"","notes":"","createdAt":""}],
            "vehicles":[
                {"id":"v1","clientId":"c","plate":"AF123BC","make":"Auto","model":"Uno","year":2021,"mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""},
                {"id":"v2","clientId":"c","plate":"AG234CD","make":"Auto","model":"Dos","year":2021,"mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}
            ],
            "orders":[],
            "settings":{"garageName":"Taller","currency":"ARS","demo":false}
        });
        let mut data:AppData=serde_json::from_value(doc).unwrap();
        write(&mut conn,&data,0).unwrap();
        let old=data.vehicles[0].plate.clone();
        data.vehicles[0].plate=data.vehicles[1].plate.clone();
        data.vehicles[1].plate=old;
        write(&mut conn,&data,1).unwrap();
        let saved=read(&conn).unwrap();
        assert_eq!(saved.revision,2);
        assert_eq!(saved.data.vehicles[0].plate,"AG234CD");
        assert_eq!(saved.data.vehicles[1].plate,"AF123BC");
        let fk:i64=conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check",[],|r|r.get(0)).unwrap();
        assert_eq!(fk,0);
        // Fail while performing the provisional plate release: SQLite must roll
        // back all work and leave the previous state and revision unchanged.
        conn.execute_batch("
          CREATE TRIGGER fail_plate_release BEFORE UPDATE OF plate ON vehicles
          WHEN new.plate LIKE 'TMP%'
          BEGIN SELECT RAISE(ABORT,'injected failure'); END;
        ").unwrap();
        data.vehicles[0].plate="AH345DE".into();
        assert!(write(&mut conn,&data,2).is_err());
        let still=read(&conn).unwrap();
        assert_eq!(still.revision,2);
        assert_eq!(still.data.vehicles[0].plate,"AG234CD");
    }
    #[test]
    fn rejects_foreign_keys_before_mutation() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        let mut data = AppData::default();
        data.vehicles.push(Vehicle {
            id: "v1".into(), client_id: "missing".into(), plate: "AF123BC".into(),
            make: "Ford".into(), model: "Ka".into(), year: serde_json::json!(2019),
            mileage: 10.0, vin: "".into(), color: "".into(), next_service_date: "".into(),
            notes: "".into(), created_at: "2026-10-21".into(),
        });
        assert!(write(&mut db, &data, 0).is_err());
        assert_eq!(read(&db).unwrap().revision, 0);
    }
    #[test]
    fn sql_projection_tampering_is_rejected_even_when_sqlite_integrity_says_ok(){
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db.execute_batch(SCHEMA).unwrap();
        let data:AppData=serde_json::from_value(serde_json::json!({
          "clients":[{"id":"c","name":"Cliente sintético","phone":"","email":"","notes":"","createdAt":""}],
          "vehicles":[{"id":"v","clientId":"c","plate":"AF123BC","make":"Marca","model":"Modelo",
              "year":2021,"mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[{"id":"o","vehicleId":"v","title":"Diagnóstico","category":"","status":"done",
              "openedAt":"","completedAt":"","mileage":0,"description":"",
              "partsCost":10.10,"laborCost":20.20,"charged":100.00,"paid":23.45,
              "notes":"","createdAt":""}],
          "settings":{"garageName":"Taller QA","currency":"ARS","demo":false}
        })).unwrap();
        write(&mut db,&data,0).unwrap();
        assert_eq!(read(&db).unwrap().revision,1);
        // SQL accepts a different monetary projection because all CHECK/FK
        // constraints are satisfied; our semantic reader must fail closed.
        db.execute("UPDATE orders SET charged_cents=9999 WHERE id='o'",[]).unwrap();
        let check:String=db.query_row("PRAGMA integrity_check",[],|r|r.get(0)).unwrap();
        assert_eq!(check,"ok");
        assert!(read(&db).err().expect("projection mismatch must be rejected").contains("inconsistentes"));
        // A caller cannot silently heal or overwrite corrupted SQL with a
        // logically different JSON snapshot without revision checks.
        db.execute("UPDATE orders SET charged_cents=10000 WHERE id='o'",[]).unwrap();
        assert_eq!(read(&db).unwrap().revision,1);
        db.execute("UPDATE vehicles SET plate='AG234CD' WHERE id='v'",[]).unwrap();
        assert!(read(&db).err().expect("projection mismatch must be rejected").contains("inconsistentes"));
        db.execute("UPDATE vehicles SET plate='AF123BC' WHERE id='v'",[]).unwrap();
        db.execute("UPDATE clients SET name='Otra persona' WHERE id='c'",[]).unwrap();
        assert!(read(&db).err().expect("projection mismatch must be rejected").contains("inconsistentes"));
        db.execute("UPDATE clients SET name='Cliente sintético' WHERE id='c'",[]).unwrap();
        assert_eq!(read(&db).unwrap().revision,1);
    }
    #[test]
    fn financial_ledger_validation_never_partially_mutates_persisted_state(){
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db.execute_batch(SCHEMA).unwrap();
        let fixture=serde_json::json!({
          "clients":[{"id":"c","name":"Cliente ficticio","phone":"","email":"","notes":"","createdAt":""}],
          "vehicles":[{"id":"v","clientId":"c","plate":"AF012BC","make":"Ford","model":"Ka","year":2022,"mileage":0,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
          "orders":[{"id":"o","vehicleId":"v","title":"Servicio",
             "status":"done","category":"","openedAt":"","completedAt":"","mileage":0,"description":"","notes":"","createdAt":"","charged":100.01,"paid":40.01,"partsCost":10.10,"laborCost":20.20,
             "payments":[{"id":"p","date":"2026-10-24","amount":40.01,"method":"transfer",
               "accountId":"acc","reference":"OP-ABC"}]}],
          "settings":{"garageName":"Taller ficticio","currency":"ARS","demo":false,
            "accounts":[{"id":"acc","name":"Cuenta del taller","kind":"bank","alias":"alias.de.prueba"}]}
        });
        let mut data:AppData=serde_json::from_value(fixture).unwrap();
        assert_eq!(write(&mut db,&data,0).unwrap().revision,1);
        let original=serde_json::to_value(read(&db).unwrap().data).unwrap();
        let candidates:[(&str,fn(&mut AppData));4]=[
          ("overpaid",( |d:&mut AppData| d.orders[0].paid=100.02)),
          ("fractional cents",( |d:&mut AppData| d.orders[0].payments[0].amount=40.001)),
          ("deleted linked account",( |d:&mut AppData| d.settings.accounts.clear())),
          ("ledger sum above aggregate",( |d:&mut AppData| d.orders[0].payments[0].amount=40.02)),
        ];
        for (name,mutate) in candidates{
            let mut invalid:AppData=serde_json::from_value(serde_json::to_value(&data).unwrap()).unwrap();
            mutate(&mut invalid);
            assert!(write(&mut db,&invalid,1).is_err(),"invalid case: {name}");
            let after=read(&db).unwrap();
            assert_eq!(after.revision,1,"revision changed for {name}");
            assert_eq!(serde_json::to_value(after.data).unwrap(),original,"data changed for {name}");
        }
        data.orders[0].payments[0].amount=40.00;
        assert_eq!(write(&mut db,&data,1).unwrap().revision,2);
        let after=read(&db).unwrap();
        assert_eq!(after.data.orders[0].payments[0].amount,40.00);
        assert_eq!(after.data.settings.accounts.len(),1);

        // Even a fully valid independent work cannot reuse a payment event.
        // Reject the entire transaction, including attempts at importing the
        // same legacy document twice under a different order ID.
        let persisted=serde_json::to_value(&after.data).unwrap();
        let mut copied=data.clone();
        let mut duplicate=copied.orders[0].clone();
        duplicate.id="o-copy".into();
        copied.orders.push(duplicate);
        let err=write(&mut db,&copied,2).err().expect("duplicate identity must be rejected");
        assert!(err.contains("Cobro duplicado"),"{err}");
        let snap=read(&db).unwrap();
        assert_eq!(snap.revision,2);
        assert_eq!(serde_json::to_value(snap.data).unwrap(),persisted);

        copied.orders[0].source_fingerprint="a".repeat(64);
        copied.orders[1].source_fingerprint="a".repeat(64);
        copied.orders[1].payments[0].id="p-copy".into();
        let err=write(&mut db,&copied,2).err().expect("duplicate identity must be rejected");
        assert!(err.contains("Documento de origen duplicado"),"{err}");
        let snap=read(&db).unwrap();
        assert_eq!(snap.revision,2);
        assert_eq!(serde_json::to_value(snap.data).unwrap(),persisted);

        // Distinct event/document identities are still valid and transactional.
        copied.orders[1].source_fingerprint="b".repeat(64);
        assert_eq!(write(&mut db,&copied,2).unwrap().revision,3);
        let restored=read(&db).unwrap();
        assert_eq!(restored.data.orders.len(),2);
        assert_eq!(restored.data.orders[0].payments[0].id,"p");
        assert_eq!(restored.data.orders[1].payments[0].id,"p-copy");
        // Accounting void keeps the source event for audit.
        let mut voided=restored.data;
        voided.orders[0].paid=0.01;
        voided.orders[0].payments[0].voided_at="2026-10-25".into();
        voided.orders[0].payments[0].void_reason="Pago duplicado de prueba, anulado".into();
        assert_eq!(write(&mut db,&voided,3).unwrap().revision,4);
        let record=read(&db).unwrap();
        assert_eq!(record.data.orders[0].payments[0].amount,40.00);
        assert_eq!(record.data.orders[0].payments[0].void_reason,"Pago duplicado de prueba, anulado");
        assert_eq!(record.data.orders[0].paid,0.01);
        let unchanged=serde_json::to_value(&record.data).unwrap();
        voided.orders[0].payments[0].void_reason="".into();
        assert!(write(&mut db,&voided,4).is_err());
        let after=read(&db).unwrap();
        assert_eq!(after.revision,4);
        assert_eq!(serde_json::to_value(after.data).unwrap(),unchanged);
    }

    #[test]
    fn existing_v1_sqlite_file_reopens_without_data_loss_and_future_version_is_rejected(){
        let path=std::env::temp_dir().join(format!("autotrace-v1-compat-{}.sqlite3",uuid::Uuid::new_v4()));
        let payload=serde_json::json!({
            "clients":[{"id":"c","name":"Cliente de prueba","phone":"","email":"","notes":"","createdAt":""}],
            "vehicles":[{"id":"v","clientId":"c","plate":"AF012BC","make":"Ford","model":"Ka",
              "year":2021,"mileage":1234.5,"vin":"","color":"","nextServiceDate":"","notes":"","createdAt":""}],
            "orders":[{"id":"o","vehicleId":"v","title":"Servicio anterior","category":"Mecánica","status":"done",
              "openedAt":"","completedAt":"","mileage":1234.5,"description":"","partsCost":33.33,
              "laborCost":11.11,"charged":99.99,"paid":20.20,"notes":"","createdAt":""}],
            "settings":{"garageName":"Taller histórico","currency":"ARS","demo":false}
        });
        let original:AppData=serde_json::from_value(payload).unwrap();
        {
            let mut legacy=Connection::open(&path).unwrap();
            legacy.pragma_update(None,"foreign_keys","ON").unwrap();
            legacy.execute_batch(SCHEMA).unwrap();
            write(&mut legacy,&original,0).unwrap();
            assert_eq!(legacy.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        }
        // Simulates installing the new binary while retaining the old local database file.
        {
            let mut reopened=open(&path).unwrap();
            let snapshot=read(&reopened).unwrap();
            assert_eq!(snapshot.revision,1);
            assert_eq!(serde_json::to_value(&snapshot.data).unwrap(),serde_json::to_value(&original).unwrap());
            assert_eq!(write(&mut reopened,&snapshot.data,snapshot.revision).unwrap().revision,2);
            assert_eq!(reopened.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        }
        {
            let raw=Connection::open(&path).unwrap();
            raw.execute_batch("PRAGMA user_version=999;").unwrap();
        }
        assert!(open(&path).is_err(),"future schema must fail closed, not auto-downgrade");
        let unmodified=Connection::open(&path).unwrap();
        assert_eq!(unmodified.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),999);
        assert_eq!(unmodified.query_row("SELECT COUNT(*) FROM orders",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        drop(unmodified);
        fs::remove_file(path).unwrap();
    }

    /// Measure the current Rust test process rather than assuming SQLite
    /// latency implies a safe memory footprint. PeakWorkingSet64 is a Windows
    /// OS counter and does not include WebView2's separate renderer process.
    #[cfg(windows)]
    fn windows_peak_working_set_mib()->Result<u64,String>{
        let query=format!("(Get-Process -Id {}).PeakWorkingSet64",std::process::id());
        let output=std::process::Command::new("powershell")
            .args(["-NoProfile","-NonInteractive","-Command",query.as_str()])
            .output().map_err(|e|format!("PowerShell memory profiler unavailable: {e}"))?;
        if !output.status.success(){return Err("Windows could not return the process memory counter".into());}
        let raw=std::str::from_utf8(&output.stdout).map_err(|e|e.to_string())?.trim();
        let bytes=raw.parse::<u64>().map_err(|e|format!("Invalid Windows peak working set: {e}"))?;
        Ok(bytes.div_ceil(1024*1024))
    }
    /// E6 synthetic storage regression budget. The Windows Actions CI invokes
    /// this ignored-by-default profile explicitly. Bounds prevent catastrophic
    /// slowdowns but are not desktop UI or customer-hardware latency SLAs.
    /// Run: cargo test --locked release_scale_profile -- --ignored --nocapture
    #[test]
    #[ignore = "manual 1k/10k/50k hardware timing; never assert CI runner latency"]
    fn release_scale_profile(){
        use std::time::Instant;
        for n in [1_000usize,10_000,50_000]{
            let mut data=AppData::default();
            data.clients.push(Client{
                id:"benchmark-client".into(),name:"Cliente ficticio".into(),phone:"".into(),
                email:"".into(),notes:"".into(),created_at:"".into()
            });
            data.vehicles.push(Vehicle{
                id:"benchmark-vehicle".into(),client_id:"benchmark-client".into(),
                plate:"AF123BC".into(),make:"Marca".into(),model:"Modelo".into(),
                year:serde_json::json!(2020),mileage:100.0,vin:"".into(),
                color:"".into(),next_service_date:"".into(),notes:"".into(),
                created_at:"".into()
            });
            for i in 0..n {
                data.orders.push(Order{
                    id:format!("synthetic-{i}"),vehicle_id:"benchmark-vehicle".into(),
                    title:"Diagnóstico ficticio".into(),category:"".into(),status:"received".into(),
                    opened_at:"".into(),completed_at:"".into(),mileage:0.0,
                    description:"".into(),parts_cost:25.25,labor_cost:30.5,
                    charged:100.99,paid:20.5,notes:"".into(),created_at:"".into(),
                    items:Vec::new(),document_kind:"".into(),document_number:"".into(),
                    source_fingerprint:"".into(),payments:Vec::new()
                });
            }
            let mut db=Connection::open_in_memory().unwrap();
            db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            db.execute_batch(SCHEMA).unwrap();
            let initial=Instant::now();
            write(&mut db,&data,0).unwrap();
            let bulk=initial.elapsed();
            let initial=Instant::now();
            let loaded=read(&db).unwrap();
            let read_time=initial.elapsed();
            assert_eq!(loaded.data.orders.len(),n);
            data.clients[0].name="Cliente actualizado".into();
            let initial=Instant::now();
            write(&mut db,&data,1).unwrap();
            let single_edit=initial.elapsed();
            assert_eq!(read(&db).unwrap().data.clients[0].name,"Cliente actualizado");
            #[cfg(windows)]
            if n==50_000 {
                // The usual debug runner measurement is approximately
                // 2s initial write, 0.6s read, 1.2s small edit. Allow
                // generous headroom for CI variability but never a 10x
                // performance regression to slip through silently.
                assert!(bulk.as_secs_f64()<25.0,
                    "E6: initial 50k-order write exceeded the 25s Windows CI budget");
                assert!(read_time.as_secs_f64()<6.0,
                    "E6: 50k-order read exceeded the 6s Windows CI budget");
                assert!(single_edit.as_secs_f64()<6.0,
                    "E6: small edit at 50k orders exceeded the 6s Windows CI budget");
            }
            #[cfg(windows)]
            {
                let peak_mib=windows_peak_working_set_mib()
                    .expect("Windows CI must produce a real OS peak working-set measurement");
                println!("synthetic_orders={n} bulk_write_ms={} read_ms={} single_edit_ms={} peak_rust_working_set_mib={peak_mib}",
                    bulk.as_millis(),read_time.as_millis(),single_edit.as_millis());
                assert!(peak_mib<=1024,"A 50k synthetic-order Rust storage session must remain under 1GiB");
            }
            #[cfg(not(windows))]
            println!("synthetic_orders={n} bulk_write_ms={} read_ms={} single_edit_ms={} peak_rust_working_set_mib=not_measured",
                bulk.as_millis(),read_time.as_millis(),single_edit.as_millis());
        }
    }

}
