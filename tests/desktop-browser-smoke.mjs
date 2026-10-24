/** Real Chromium DOM smoke with a simulated Tauri command boundary.
 * Tests UI wiring only: it does NOT claim Windows vault, native files or Drive work.
 */
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
const executablePath=process.env.CHROME_PATH||'/usr/bin/google-chrome';
const base=process.env.AUTOTRACE_SMOKE_BASE||'http://127.0.0.1:8765/lab/';
const phrase='AT-'+('a'.repeat(32))+'-'+('b'.repeat(32));
const browser=await chromium.launch({headless:true,executablePath,args:['--no-sandbox','--disable-dev-shm-usage']});
const page=await browser.newPage({viewport:{width:1180,height:800}});
const errors=[];
page.on('pageerror',e=>errors.push(String(e)));
try{
  await page.addInitScript(({phrase})=>{
    const callLog=[];
    let autoUnlock=false;
    const data={clients:[],vehicles:[],orders:[],settings:{garageName:'Mi taller',currency:'ARS',demo:false}};
    window.__AUTOTRACE_TEST_CALLS__=callLog;
    window.__TAURI__={core:{invoke:async(cmd,args={})=>{
      callLog.push({cmd,args});
      if(cmd==='load_state')return {data,revision:0};
      if(cmd==='backup_status')return {lastBackupUnixMs:Date.now(),localOnly:true,externalDirectory:null,lastExternalBackupUnixMs:null};
      if(cmd==='drive_backup_status')return {directory:null,unlocked:false};
      if(cmd==='drive_api_status')return {identityStatus:'not_configured',connected:false,pendingCount:0,lastVerifiedUploadAt:null,autoUnlockEnabled:autoUnlock,developerMode:false};
      if(cmd==='generate_recovery_phrase')return phrase;
      if(cmd==='drive_api_enable_auto_unlock'){
        if(args.passphrase!==phrase)throw Error('Unexpected mock phrase');
        autoUnlock=true;return null;
      }
      if(cmd==='export_portable_encrypted_backup'){
        if(args.passphrase!==phrase)throw Error('Wrong mock phrase');
        return 'AutoTrace_test.atbackup.age';
      }
      throw Error('Unexpected native command: '+cmd);
    }}};
  },{phrase});
  await page.goto(base+'#ajustes',{waitUntil:'domcontentloaded'});
  await page.locator('[data-action="generate-recovery-kit"]').waitFor({timeout:25000});
  assert.equal(await page.getByRole('button',{name:'Conectar con Google'}).count(),0,'Cannot offer OAuth if app identity absent');
  // The safe offline recovery workflow must remain available without Google Cloud.
  await page.locator('[data-action="generate-recovery-kit"]').click();
  await page.locator('#confirm-dialog[open]').waitFor();
  await page.locator('#confirm-accept').click();
  await page.locator('#entity-form[data-kind="drive-recovery-kit"]').waitFor();
  assert.equal(await page.locator('#recovery-kit-value').inputValue(),phrase);
  await page.locator('[name="recoverySuffix"]').fill(phrase.slice(-8));
  await page.locator('[name="savedOutside"]').check();
  await page.locator('#entity-form [type="submit"]').click();
  await page.locator('#entity-form[data-kind="drive-recovery-kit"]').waitFor({state:'hidden',timeout:10000});
  const first=await page.evaluate(()=>window.__AUTOTRACE_TEST_CALLS__);
  assert.equal(first.filter(x=>x.cmd==='drive_api_enable_auto_unlock').length,1);
  assert.equal(await page.locator('#modal-content').innerText(),'','Secret dialog must be removed after success');
  await page.locator('[data-action="export-portable-backup"]').click();
  await page.locator('#entity-form[data-kind="drive-portable-export"]').waitFor();
  // Reject mismatched phrases BEFORE any native export.
  await page.locator('[name="drivePassphrase"]').fill(phrase);
  await page.locator('[name="drivePassphraseRepeat"]').fill('different-long-test-phrase');
  await page.locator('#entity-form [type="submit"]').click();
  assert.equal((await page.evaluate(()=>window.__AUTOTRACE_TEST_CALLS__)).filter(x=>x.cmd==='export_portable_encrypted_backup').length,0);
  await page.locator('[name="drivePassphraseRepeat"]').fill(phrase);
  await page.locator('#entity-form [type="submit"]').click();
  await page.locator('#entity-form[data-kind="drive-portable-export"]').waitFor({state:'hidden',timeout:10000});
  const calls=await page.evaluate(()=>window.__AUTOTRACE_TEST_CALLS__);
  assert.equal(calls.filter(x=>x.cmd==='export_portable_encrypted_backup').length,1);
  assert.equal(await page.locator('#modal-content').innerText(),'');
  assert.deepEqual(errors,[],'Chromium must not throw runtime errors');
  console.log('Desktop mocked UI smoke OK: offline recovery kit and export, without OAuth.');
}finally{await browser.close();}
