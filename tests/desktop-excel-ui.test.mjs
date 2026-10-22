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
test('Restore and destructive changes take a backup first',()=>{
  assert.match(frontend,/if\(!await createDesktopBackup\(true\)\)throw Error/);
  assert.match(frontend,/if\(desktopInvoke && !await createDesktopBackup\(true\)\)break/);
  assert.match(backup,/run_to_completion/);
  assert.match(backup,/sqliteSha256/);
  assert.match(backup,/excelSha256/);
});
