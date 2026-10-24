import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const read = path => readFileSync(new URL('../'+path, import.meta.url),'utf8');
const ui=read('site/app.js');
const native=read('desktop/src-tauri/src/drive_backup.rs');
const commands=read('desktop/src-tauri/src/lib.rs');
const backup=read('desktop/src-tauri/src/backup.rs');

test('Portable backup creates a fresh verified dual snapshot and seals it before external write',()=>{
  assert.match(native,/pub\(crate\) async fn export_portable_encrypted_backup/);
  assert.match(native,/backup::backup_pair\(&crate::store::open_app\(app\)\?/);
  assert.match(native,/seal\(&pair,&cipher_dir,/);
  assert.match(native,/std::io::copy\(&mut from,&mut out\)/);
  assert.match(native,/if hash\(&temp\)\?!=hash\(&cipher\)\?/);
  assert.match(native,/El archivo de destino ya existe/);
  assert.match(backup,/sqliteSha256/);
  assert.match(backup,/excelSha256/);
  assert.match(commands,/drive_backup::export_portable_encrypted_backup/);
  assert.match(ui,/export_portable_encrypted_backup/);
  assert.match(ui,/Exportar copia cifrada a USB/);
});
test('Portable recovery phrase is generated in native code and never placed in global JS state',()=>{
  assert.match(native,/fn generate_recovery_phrase\(\)->String/);
  assert.match(native,/uuid::Uuid::new_v4\(\)\.simple\(\).*uuid::Uuid::new_v4\(\)\.simple\(\)/);
  assert.match(commands,/drive_backup::generate_recovery_phrase/);
  assert.match(ui,/Crear kit de recuperación/);
  assert.doesNotMatch(ui,/window\.__autotraceRecoveryKitSuffix/);
});
