-- AutoTrace Desktop v1: IDs y contenido compatible con JSON de la PWA.
-- Los campos queryables de dominio quedan indexados junto con el registro validado.
CREATE TABLE IF NOT EXISTS clients (
  id TEXT PRIMARY KEY NOT NULL CHECK(length(id) BETWEEN 1 AND 100),
  name TEXT NOT NULL CHECK(length(trim(name)) > 0),
  record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE TABLE IF NOT EXISTS vehicles (
  id TEXT PRIMARY KEY NOT NULL CHECK(length(id) BETWEEN 1 AND 100),
  client_id TEXT NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
  plate TEXT NOT NULL UNIQUE CHECK(length(plate) BETWEEN 5 AND 12 AND plate = upper(plate)),
  record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE INDEX IF NOT EXISTS idx_vehicles_client ON vehicles(client_id);
CREATE TABLE IF NOT EXISTS orders (
  id TEXT PRIMARY KEY NOT NULL CHECK(length(id) BETWEEN 1 AND 100),
  vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
  status TEXT NOT NULL CHECK(status IN ('received', 'progress', 'waiting', 'done', 'delivered')),
  charged_cents INTEGER NOT NULL CHECK(charged_cents >= 0),
  paid_cents INTEGER NOT NULL CHECK(paid_cents >= 0 AND paid_cents <= charged_cents),
  parts_cents INTEGER NOT NULL CHECK(parts_cents >= 0),
  labor_cents INTEGER NOT NULL CHECK(labor_cents >= 0),
  record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE INDEX IF NOT EXISTS idx_orders_vehicle ON orders(vehicle_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE TABLE IF NOT EXISTS app_settings (
  id INTEGER PRIMARY KEY NOT NULL CHECK(id = 1),
  record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY NOT NULL,
  int_value INTEGER NOT NULL CHECK(int_value >= 0)
);
INSERT OR IGNORE INTO app_meta(key, int_value) VALUES('revision', 0);
PRAGMA user_version = 1;
