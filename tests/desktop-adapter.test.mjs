import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const app=readFileSync(new URL('../site/app.js', import.meta.url),'utf8');
const config=JSON.parse(readFileSync(new URL('../desktop/src-tauri/tauri.conf.json', import.meta.url)));
const permissions=JSON.parse(readFileSync(new URL('../desktop/src-tauri/capabilities/default.json', import.meta.url)));

test('Tauri desktop reuses site frontend and exposes only native commands',()=>{
  assert.equal(config.build.frontendDist,'../../site');
  assert.equal(config.app.withGlobalTauri,true);
  assert.deepEqual(permissions.permissions,['core:default']);
});
test('desktop adapter is explicit, never seeds demo, checks revision',()=>{
  assert.match(app,/desktopInvoke\('load_state'\)/);
  assert.match(app,/desktopInvoke\('save_state',\{data:canonical,expectedRevision:desktopRevision\}\)/);
  assert.match(app,/if\(!desktopInvoke && 'serviceWorker'/);
  const desktopBranch=app.slice(app.indexOf('async function loadData(){'),app.indexOf('  try {',app.indexOf('async function loadData(){')));
  assert.doesNotMatch(desktopBranch,/seedDemo\(/);
});
test('shell requires no network / no external scripts',()=>{
  assert.equal(config.app.security.csp.includes("script-src 'self'"),true);
  assert.equal(config.bundle.targets[0],'nsis');
});
