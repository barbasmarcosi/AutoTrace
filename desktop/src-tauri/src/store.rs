use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf, time::Duration};
use tauri::Manager;

const SCHEMA: &str = include_str!("../migrations/001_init.sql");
const MAX_RECORDS: (usize, usize, usize) = (15_000, 30_000, 100_000);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Client {
    id: String,
    name: String,
    phone: String,
    email: String,
    notes: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Vehicle {
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
struct Order {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    garage_name: String,
    currency: String,
    demo: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { garage_name: "Mi taller".into(), currency: "ARS".into(), demo: false }
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
    data: AppData,
    revision: u64,
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
        if !orders.insert(o.id.as_str()) { return Err("Trabajo duplicado".into()); }
    }
    Ok(())
}

fn current_revision(conn: &Connection) -> Result<u64, String> {
    let val: i64 = conn.query_row("SELECT int_value FROM app_meta WHERE key='revision'", [], |r| r.get(0)).map_err(sqlite_error)?;
    u64::try_from(val).map_err(sqlite_error)
}

pub(crate) fn read(conn: &Connection) -> Result<Snapshot, String> {
    let mut data = AppData::default();
    for (sql, target) in [
        ("SELECT record_json FROM clients ORDER BY rowid", 0),
        ("SELECT record_json FROM vehicles ORDER BY rowid", 1),
        ("SELECT record_json FROM orders ORDER BY rowid", 2),
    ] {
        let mut stmt = conn.prepare(sql).map_err(sqlite_error)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(sqlite_error)?;
        for row in rows {
            let json = row.map_err(sqlite_error)?;
            match target {
                0 => data.clients.push(serde_json::from_str(&json).map_err(sqlite_error)?),
                1 => data.vehicles.push(serde_json::from_str(&json).map_err(sqlite_error)?),
                _ => data.orders.push(serde_json::from_str(&json).map_err(sqlite_error)?),
            }
        }
    }
    if let Some(raw) = conn.query_row("SELECT record_json FROM app_settings WHERE id=1", [], |r| r.get::<_, String>(0)).optional().map_err(sqlite_error)? {
        data.settings = serde_json::from_str(&raw).map_err(sqlite_error)?;
    }
    validate(&data)?;
    Ok(Snapshot { data, revision: current_revision(conn)? })
}

fn write(conn: &mut Connection, data: &AppData, expected_revision: u64) -> Result<SaveResult, String> {
    validate(data)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sqlite_error)?;
    let existing = current_revision(&tx)?;
    if existing != expected_revision {
        return Err("Otra ventana modificó los registros. Cerrá y volvé a abrir antes de guardar.".into());
    }
    tx.execute("DELETE FROM orders", []).map_err(sqlite_error)?;
    tx.execute("DELETE FROM vehicles", []).map_err(sqlite_error)?;
    tx.execute("DELETE FROM clients", []).map_err(sqlite_error)?;
    {
        let mut stmt = tx.prepare("INSERT INTO clients(id,name,record_json) VALUES (?1,?2,?3)").map_err(sqlite_error)?;
        for c in &data.clients {
            stmt.execute(params![c.id, c.name, serde_json::to_string(c).map_err(sqlite_error)?]).map_err(sqlite_error)?;
        }
    }
    {
        let mut stmt = tx.prepare("INSERT INTO vehicles(id,client_id,plate,record_json) VALUES (?1,?2,?3,?4)").map_err(sqlite_error)?;
        for v in &data.vehicles {
            stmt.execute(params![v.id, v.client_id, v.plate, serde_json::to_string(v).map_err(sqlite_error)?]).map_err(sqlite_error)?;
        }
    }
    {
        let mut stmt = tx.prepare("INSERT INTO orders(id,vehicle_id,status,charged_cents,paid_cents,parts_cents,labor_cents,record_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(sqlite_error)?;
        for o in &data.orders {
            stmt.execute(params![o.id, o.vehicle_id, o.status,
                cents(o.charged, "importe")?, cents(o.paid, "pagado")?,
                cents(o.parts_cost, "repuestos")?, cents(o.labor_cost, "mano de obra")?,
                serde_json::to_string(o).map_err(sqlite_error)?]).map_err(sqlite_error)?;
        }
    }
    tx.execute("INSERT INTO app_settings(id,record_json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET record_json=excluded.record_json", [serde_json::to_string(&data.settings).map_err(sqlite_error)?]).map_err(sqlite_error)?;
    let next = existing.checked_add(1).ok_or("Se agotó la revisión de datos")?;
    tx.execute("UPDATE app_meta SET int_value=?1 WHERE key='revision'", [next]).map_err(sqlite_error)?;
    tx.commit().map_err(sqlite_error)?;
    Ok(SaveResult { revision: next })
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
}
