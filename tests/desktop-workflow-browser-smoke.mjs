/**
 * Real Chromium integration of the Desktop shell with a stateful, strict
 * synthetic Tauri RPC boundary. Native SQLite is separately covered by Rust
 * Windows tests; this test verifies UI -> invoke -> persisted revision -> UI
 * after a full reload without IndexedDB or a demo seed.
 */
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {validateData} from '../site/model.mjs';

const base=process.env.AUTOTRACE_SMOKE_BASE||'http://127.0.0.1:8765/lab/';
const executablePath=process.env.CHROME_PATH||'/usr/bin/google-chrome';
const state={
  revision:0,
  data:validateData({clients:[],vehicles:[],orders:[],
    settings:{garageName:'Taller de prueba',currency:'ARS',demo:false}}),
  saveCalls:0,backupCalls:0,pdfExports:[],
};
let rejectNextSave=false;
const rpc=async(cmd,args={})=>{
  switch(cmd){
    case 'load_state':return structuredClone({data:state.data,revision:state.revision});
    case 'save_state':{
      state.saveCalls++;
      if(rejectNextSave){rejectNextSave=false;throw Error('Conflicto de revisión: recargá el estado antes de editar.');}
      if(args.expectedRevision!==state.revision)throw Error('Conflicto de revisión: datos desactualizados.');
      const verified=validateData(args.data);
      state.data=structuredClone(verified);state.revision++;
      return {revision:state.revision};
    }
    case 'backup_status':return {lastBackupUnixMs:Date.now(),lastExternalBackupUnixMs:null,externalDirectory:null};
    case 'drive_backup_status':return {directory:null,unlocked:false};
    case 'drive_api_status':return {connected:false,identityStatus:'not_configured',pendingCount:0,
      lastVerifiedUploadAt:null,autoUnlockEnabled:false,developerMode:false};
    case 'save_customer_pdf':{
      assert.match(args.filename,/^AutoTrace_(presupuesto|remito)_AF012BC_.*\.pdf$/);
      const binary=Buffer.from(args.bytes);
      assert.ok(binary.subarray(0,9).toString('latin1')==='%PDF-1.4\n');
      state.pdfExports.push({name:args.filename,size:binary.length});
      return true;
    }
    case 'backup_now':state.backupCalls++;throw Error('Unexpected automatic backup in a fresh-session smoke');
    default:throw Error('Unexpected Desktop RPC: '+cmd);
  }
};
const browser=await chromium.launch({headless:true,executablePath,args:['--no-sandbox','--disable-dev-shm-usage']});
const page=await browser.newPage({viewport:{width:1180,height:800}});
const failures=[];
page.on('pageerror',e=>failures.push('pageerror: '+e.message));
page.on('console',m=>{
  // The deliberate rejected revision must be surfaced to the operator. The
  // frontend logs that expected failure, but all other console errors fail CI.
  if(m.type()==='error'&&!m.text().includes('Conflicto de revisión'))
    failures.push('console: '+m.text());
});
page.on('requestfailed',r=>failures.push('HTTP: '+r.url()+' '+r.failure()?.errorText));
await page.exposeFunction('__autoTraceTestNativeRpc',rpc);
await page.addInitScript(()=>{
  window.__TAURI__={core:{invoke:(cmd,args)=>window.__autoTraceTestNativeRpc(cmd,args)}};
});
const modal=kind=>page.locator('#entity-form[data-kind="'+kind+'"]');
const save=async(kind)=>{
  await modal(kind).locator('[type="submit"]').click();
  await modal(kind).waitFor({state:'hidden',timeout:10000});
};
try {
  await page.goto(base+'#clientes',{waitUntil:'domcontentloaded'});
  await page.locator('[data-action="new-client"]').first().click();
  await modal('client').waitFor();
  await page.locator('#f-name').fill('Lucía de Prueba');
  await page.locator('#f-phone').fill('0011223344');
  await save('client');
  assert.equal(state.data.clients.length,1);
  assert.equal(state.data.clients[0].phone,'0011223344');
  const clientId=state.data.clients[0].id;
  assert.ok(!clientId.startsWith('demo-'),'Desktop must never load PWA fixtures');

  await page.locator('[data-action="new-vehicle"][data-client="'+clientId+'"]').click();
  await modal('vehicle').waitFor();
  assert.equal(await page.locator('#f-clientId').inputValue(),clientId);
  await page.locator('#f-plate').fill('AF012BC');
  await page.locator('#f-make').fill('Toyota');
  await page.locator('#f-model').fill('Corolla');
  await page.locator('#f-mileage').fill('125000');
  await save('vehicle');
  assert.equal(state.data.vehicles.length,1);
  assert.equal(state.data.vehicles[0].clientId,clientId);
  const vehicleId=state.data.vehicles[0].id;

  await page.locator('[data-action="new-order"][data-vehicle="'+vehicleId+'"]').first().click();
  await modal('order').waitFor();
  assert.equal(await page.locator('#f-vehicleId').inputValue(),vehicleId);
  await page.locator('#f-title').fill('Revisión de frenos');
  await page.locator('[data-action="add-order-line"]').click();
  await page.locator('[name="lineDescription"]').fill('Mano de obra - frenos');
  await page.locator('[name="lineKind"]').selectOption('labor');
  await page.locator('[name="lineQuantity"]').fill('1');
  await page.locator('[name="lineUnitPrice"]').fill('12500.25');
  await page.locator('[name="lineUnitPrice"]').dispatchEvent('input');
  assert.equal(await page.locator('#f-charged').inputValue(),'12500.25');
  await save('order');
  assert.equal(state.data.orders.length,1);
  const orderId=state.data.orders[0].id;
  assert.equal(state.data.orders[0].charged,12500.25);
  assert.equal(state.data.orders[0].items.length,1);
  for(const action of ['export-pdf-quote','export-pdf-work']){
    await page.locator('[data-action="'+action+'"]').first().click();
  }
  assert.equal(state.pdfExports.length,2,'Both PDFs must pass through a native save-file dialog command');
  assert.ok(state.pdfExports.every(p=>p.size>500));
  await page.locator('[data-action="add-payment"]').first().click();
  await modal('payment').waitFor();
  await page.locator('[name="payment"]').fill('5000.25');
  await page.locator('[name="method"]').selectOption('transfer');
  await page.locator('[name="accountName"]').fill('Banco ficticio');
  await page.locator('[name="paymentReference"]').fill('PRUEBA-123');
  await save('payment');
  assert.equal(state.data.orders[0].paid,5000.25);
  assert.equal(state.data.orders[0].payments.length,1);
  assert.equal(state.data.settings.accounts.length,1);
  const paymentId=state.data.orders[0].payments[0].id;
  assert.equal(state.data.orders[0].payments[0].accountId,state.data.settings.accounts[0].id);

  await page.locator('[data-action="void-payment"][data-payment-id="'+paymentId+'"]').click();
  await modal('void-payment').waitFor();
  await page.locator('[name="voidReason"]').fill('Asiento duplicado ingresado durante una prueba de caja.');
  await save('void-payment');
  assert.equal(state.data.orders[0].paid,0);
  assert.equal(state.data.orders[0].payments[0].amount,5000.25);
  assert.match(state.data.orders[0].payments[0].voidReason,/Asiento duplicado/);
  assert.equal(state.data.orders[0].payments[0].voidedAt.length,10);
  assert.equal(await page.locator('[data-action="void-payment"][data-payment-id="'+paymentId+'"]').count(),0);
  assert.equal(state.backupCalls,0);

  // Full Desktop reload: the native boundary retains records and revision.
  const committed=state.revision;
  await page.reload({waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>document.querySelector('#app')?.textContent?.includes('Revisión de frenos'));
  assert.equal(state.revision,committed,'Reopening must not write or seed demo data');
  assert.match(await page.locator('#app').innerText(),/ANULADO/);

  // Reject one stale transaction, keep the edit dialog visible, then prove
  // no mutation or optimistic version advance occurred.
  await page.locator('[data-action="edit-order"]').first().click();
  await modal('order').waitFor();
  await page.locator('#f-title').fill('No debe persistirse');
  rejectNextSave=true;
  const before=structuredClone(state.data);
  await modal('order').locator('[type="submit"]').click();
  await page.locator('#toast-region .toast.error').last().waitFor();
  assert.deepEqual(state.data,before,'Stale saves may never mutate persisted state');
  assert.equal(state.revision,committed,'Rejected save must not advance persisted revision');
  assert.equal(await modal('order').count(),1,'Unsaved form should remain open on error');

  // Desktop UX at narrow width: no clipped editor, modal remains keyboard-safe.
  await page.setViewportSize({width:390,height:720});
  const layout=await page.evaluate(()=>{
    const dialog=document.querySelector('#form-dialog');
    const r=dialog.getBoundingClientRect();
    return {open:dialog.open,left:r.left,right:r.right,width:innerWidth};
  });
  assert.equal(layout.open,true);
  assert.ok(layout.left>=-1 && layout.right<=layout.width+1,
    'Desktop at narrow window must not clip the form');
  assert.deepEqual(failures,[],'Desktop browser flow must not produce runtime exceptions');
  console.log('PASS: desktop-mode Chromium CRUD, cent-accurate ledger, reload, stale revision and mobile modal');
} finally {await browser.close();}
