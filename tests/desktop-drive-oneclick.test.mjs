import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read=p=>readFileSync(new URL('../'+p,import.meta.url),'utf8');
const ui=read('site/app.js');
const native=read('desktop/src-tauri/src/drive_api.rs');
const crypto=read('desktop/src-tauri/src/drive_backup.rs');
const runtime=read('desktop/src-tauri/src/lib.rs');
const transport=read('desktop/src-tauri/src/drive_transport.rs');

test('Google OAuth public identity is embedded by native build, never required of workshop user',()=>{
  assert.match(native,/option_env!\("AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID"\)/);
  assert.match(native,/if !cfg!\(debug_assertions\)/);
  assert.match(native,/identity_status:identity/);
  assert.match(native,/migration_required/);
  const block=ui.slice(ui.indexOf('function renderDirectDrive(){'),ui.indexOf('function renderSettings(){'));
  assert.match(block,/Conectar con Google/);
  assert.match(block,/!connected&&ready/);
  assert.match(block,/st\.developerMode&&!ready/);
  assert.match(block,/todavía no está activada/);
  assert.doesNotMatch(block,/OAuth Client ID de Google Cloud/);
});

test('optional auto-unlock goes through the Windows keyring; portable phrase stays a requirement',()=>{
  assert.match(crypto,/keyring::Entry::new\(WINDOWS_KEY_SERVICE,WINDOWS_KEY_ACCOUNT\)/);
  assert.match(crypto,/pub\(crate\) fn phrase_for_backup/);
  assert.match(crypto,/pub\(crate\) fn disable_auto_unlock/);
  assert.match(native,/unlock_for_api\(passphrase\.clone\(\),&existing\)/);
  assert.match(ui,/Repetir frase de recuperación/);
  assert.match(ui,/Windows protegerá tu frase/);
  for (const command of ['drive_api_enable_auto_unlock','drive_api_disable_auto_unlock']){
    assert.match(runtime,new RegExp('drive_api::'+command));
    assert.match(ui,new RegExp(command));
  }
});

test('Drive sync keeps ciphertext locally, confirms remote checksum and records only safe states',()=>{
  assert.match(native,/drive_backup::seal\(result,&dir,&phrase\)/);
  assert.match(native,/last_sync_state:Option<String>/);
  assert.match(native,/classify_sync_failure/);
  assert.match(native,/pending\(app\)\?/);
  assert.match(transport,/md5Checksum/);
  assert.match(transport,/file_md5/);
  assert.match(transport,/resumable/);
  assert.match(transport,/fn mock_google_http_errors_are_safe/);
});

test('legacy Drive-for-Desktop remains behind advanced UI and PWA never invokes native API',()=>{
  assert.match(ui,/Opciones avanzadas · método anterior de carpeta sincronizada/);
  assert.match(ui,/if\(!desktopInvoke\)return '';/);
  assert.match(ui,/Los respaldos locales SQLite y Excel siguen SIN CIFRAR/);
  assert.match(ui,/if\(!desktopInvoke && 'serviceWorker'/);
});
