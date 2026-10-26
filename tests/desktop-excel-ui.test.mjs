import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const frontend=readFileSync(new URL('../site/app.js',import.meta.url),'utf8');
const runtime=readFileSync(new URL('../desktop/src-tauri/src/lib.rs',import.meta.url),'utf8');
const backup=readFileSync(new URL('../desktop/src-tauri/src/backup.rs',import.meta.url),'utf8');
test('Excel commands are desktop-only and wired to visible actions',()=>{
  for(const action of ['export-excel-desktop','import-excel-desktop','backup-desktop-now'])assert.ok(frontend.includes(action));
  for(const command of ['backup_now','backup_status','export_excel_dialog','choose_excel_import']){
    assert.ok(runtime.includes(`backup::${command}`));assert.ok(frontend.includes(`'${command}'`));
  }
  assert.match(frontend,/if\(desktopInvoke\)/);
});
test('The client-facing Excel command is desktop-only and separate from private backup',()=>{
  assert.match(frontend,/Excel para cliente/);
  assert.match(frontend,/case 'export-client-excel'/);
  assert.match(runtime,/mod client_report;/);
  assert.match(runtime,/backup::export_customer_excel_dialog/);
});
test('SQLite recovery is checked native-side, and shown only on desktop',()=>{
  assert.match(frontend,/restore-sqlite-desktop/);
  assert.match(frontend,/choose_sqlite_import/);
  assert.match(runtime,/backup::choose_sqlite_import/);
  assert.match(backup,/SQLITE_OPEN_READ_ONLY/);
  assert.match(backup,/PRAGMA user_version/);
});
test('User can test recovery without mutating live SQLite',()=>{
  assert.match(frontend,/test-backup-recovery/);
  assert.match(frontend,/test_backup_recovery/);
  assert.match(runtime,/backup::test_backup_recovery/);
  assert.match(backup,/fn recovery_rehearsal/);
  assert.match(backup,/Connection::open_in_memory/);
  assert.match(backup,/fn check_distinct_volume/);
});
test('Legacy workbook import and client document export are connected only on desktop',()=>{
  assert.match(frontend,/choose-workshop-logo/);
  assert.match(runtime,/legacy_documents::choose_workshop_logo/);
  assert.match(frontend,/import-legacy-document/);
  assert.match(frontend,/choose_legacy_document/);
  assert.match(frontend,/export-legacy-quote/);
  assert.match(frontend,/export_legacy_document/);
  assert.match(runtime,/legacy_documents::choose_legacy_document/);
  assert.match(runtime,/legacy_documents::export_legacy_document/);
});
test('Quotes are excluded from financial listings and CSV exports, with separate quote box',()=>{
  assert.match(frontend,/let orders=db\.orders\.filter\(o=>o\.documentKind!=='quote'\)/);
  assert.match(frontend,/const rows=db\.orders\.filter\(o=>o\.documentKind!=='quote'\)/);
  assert.match(frontend,/if\(o\.documentKind==='quote'\)/);
  assert.match(frontend,/Estimación informativa: no es una cuenta por cobrar/);
});
test('Restore and destructive changes take a backup first',()=>{
  assert.match(frontend,/if\(!await createDesktopBackup\(true\)\)throw Error/);
  assert.match(frontend,/if\(desktopInvoke && !await createDesktopBackup\(true\)\)break/);
  assert.match(backup,/run_to_completion/);
  assert.match(backup,/sqliteSha256/);
  assert.match(backup,/excelSha256/);
});

test('Customer PDFs are offline and default actions in PWA and native Windows',()=>{
  for(const action of ['export-pdf-quote','export-pdf-work','Presupuesto Excel','Remito Excel'])
    assert.ok(frontend.includes(action),'Missing document action: '+action);
  assert.match(frontend,/writeWorkshopPdf\(db,orderId,kind\)/);
  assert.match(frontend,/desktopInvoke\('save_customer_pdf'/);
  assert.match(runtime,/customer_pdf::save_customer_pdf/);
  const native=readFileSync(new URL('../desktop/src-tauri/src/customer_pdf.rs',import.meta.url),'utf8');
  assert.match(native,/\.save_file\(\)/);
  assert.match(native,/sync_all\(\)/);
  assert.match(native,/El archivo PDF ya existe/);
  const worker=readFileSync(new URL('../site/sw.js',import.meta.url),'utf8');
  assert.match(worker,/workshop-pdf\.mjs/);
});
