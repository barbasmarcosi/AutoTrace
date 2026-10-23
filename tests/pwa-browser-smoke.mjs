/** Browser-level smoke with a real Chrome and persisted IndexedDB. */
import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const XLSX=require('../site/vendor/xlsx.mini.min.js');
function syntheticDocument({customer='Cliente Nuevo Laboratorio',phone='1144443333',plate='AK112BC',make='Honda',model='Fit',id='TEST-LAB-2026'}={}){
  const wb=XLSX.utils.book_new();
  const ws=XLSX.utils.aoa_to_sheet([[]]);
  const put=(ref,val)=>{ws[ref]={t:typeof val==='number'?'n':'s',v:val};};
  put('B1','REMITO TRABAJO');put('D4',id);put('J4',46317);
  put('I6',customer);put('I7',phone);put('D12',make);put('D13',model);put('D14',plate);
  put('B18','REFERENCIA / DETALLE PIEZAS');put('H18','CANTIDAD');put('I18','VALOR UNITARIO');
  put('B20','Filtro laboratorio');put('H20',2);put('I20',5000);put('J20',10000);
  put('B21','Mano de obra');put('H21',1);put('I21',15000);put('J21',15000);
  put('I46','TOTAL');put('J46',25000);ws['!ref']='B1:J46';
  XLSX.utils.book_append_sheet(wb,ws,'Hoja1');
  return Buffer.from(XLSX.write(wb,{type:'buffer',bookType:'xlsx'}));
}
async function inspectStore(page){
  return page.evaluate(async()=>new Promise((resolve,reject)=>{
    const req=indexedDB.open('autotrace-lab-v1');
    req.onerror=()=>reject(req.error);
    req.onsuccess=()=>{
      const db=req.result,read=db.transaction('application','readonly').objectStore('application').get('records');
      read.onsuccess=()=>{resolve(read.result);db.close();};
      read.onerror=()=>reject(read.error);
    };
  }));
}

const base=process.env.AUTOTRACE_SMOKE_BASE||'http://127.0.0.1:8765/lab/';
const executablePath=process.env.CHROME_PATH||'/usr/bin/google-chrome';
const browser=await chromium.launch({headless:true,executablePath,args:['--no-sandbox','--disable-dev-shm-usage']});
const errors=[];
const page=await browser.newPage({viewport:{width:390,height:844}});
page.on('console',msg=>{if(msg.type()==='error')errors.push('console: '+msg.text())});
page.on('pageerror',e=>errors.push('pageerror: '+e.stack));
page.on('requestfailed',r=>errors.push('network: '+r.url()+' '+r.failure()?.errorText));
try{
  await page.goto(base,{waitUntil:'domcontentloaded',timeout:25000});
  await page.waitForFunction(()=>{
    const root=document.querySelector('#app');
    return !!root && (!root.querySelector('.boot')) && root.textContent.length>300;
  },{timeout:25000});
  assert.match(await page.locator('#app').innerText(),/PANEL DE CONTROL|Modo demostración|Acciones rápidas|Resumen de/iu);
  const excelVersion=await page.evaluate(()=>window.XLSX?.version);
  assert.equal(excelVersion,'0.20.3','bundled XLSX offline library must initialize');
  await page.goto(base+'#ajustes',{waitUntil:'domcontentloaded',timeout:25000});
  await page.getByText('Importar presupuesto / remito').waitFor({timeout:10000});
  await page.getByText('Restaurar Excel completo').waitFor({timeout:10000});
  await page.getByText('Excel completo',{exact:false}).first().waitFor({timeout:10000});
  const dbs=await page.evaluate(async()=>indexedDB.databases?await indexedDB.databases():[]);
  assert.ok(dbs.some(x=>x.name==='autotrace-lab-v1'),'LAB must not share stable IndexedDB');
  // End-to-end: import an unknown customer+vehicle directly inside the modal.
  const before=await inspectStore(page);
  await page.locator('#import-workshop-xlsx').setInputFiles({
    name:'remito-lab-test.xlsx',mimeType:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    buffer:syntheticDocument()
  });
  await page.locator('#entity-form[data-kind="legacy"]').waitFor({timeout:15000});
  assert.equal(await page.locator('#legacy-client-select').inputValue(),'new');
  assert.equal(await page.locator('#legacy-vehicle-select').inputValue(),'new');
  await page.locator('[name="confirmLegacy"]').check();
  await page.locator('#entity-form [type="submit"]').click();
  await page.waitForURL(/#trabajo\//,{timeout:15000});
  const after=await inspectStore(page);
  assert.equal(after.clients.length,before.clients.length+1);
  assert.equal(after.vehicles.length,before.vehicles.length+1);
  assert.equal(after.orders.length,before.orders.length+1);
  const newClient=after.clients.find(c=>c.name==='Cliente Nuevo Laboratorio');
  const newCar=after.vehicles.find(c=>c.plate==='AK112BC');
  const newOrder=after.orders.find(o=>o.vehicleId===newCar?.id);
  assert.equal(newCar?.clientId,newClient?.id);
  assert.equal(newOrder?.items.length,2);
  assert.equal(newOrder?.charged,25000);
  // Register an electronic payment and its new destination without leaving the order.
  await page.locator('[data-action="add-payment"][data-id="'+newOrder.id+'"]').click();
  await page.locator('#entity-form[data-kind="payment"]').waitFor({timeout:10000});
  await page.locator('[name="payment"]').fill('10000');
  await page.locator('[name="method"]').selectOption('transfer');
  await page.locator('[name="accountName"]').fill('Banco de pruebas');
  await page.locator('[name="accountAlias"]').fill('taller.prueba');
  await page.locator('[name="paymentReference"]').fill('TRX-2026');
  await page.locator('#entity-form [type="submit"]').click();
  await page.locator('#entity-form[data-kind="payment"]').waitFor({state:'hidden',timeout:10000});
  const withPayment=await inspectStore(page);
  const updatedOrder=withPayment.orders.find(o=>o.id===newOrder.id);
  assert.equal(updatedOrder?.paid,10000);
  assert.equal(updatedOrder?.payments?.length,1);
  assert.equal(updatedOrder.payments[0].method,'transfer');
  assert.equal(updatedOrder.payments[0].reference,'TRX-2026');
  const receiver=withPayment.settings.accounts.find(a=>a.id===updatedOrder.payments[0].accountId);
  assert.equal(receiver.name,'Banco de pruebas');
  await page.goto(base+'#finanzas',{waitUntil:'domcontentloaded'});
  await page.getByText('Cobros por cuenta receptora').waitFor({timeout:10000});
  await page.getByText('Resultado mano de obra').waitFor({timeout:10000});
  // End-to-end: identify an existing known owner and plate; no duplicate records.
  await page.goto(base+'#ajustes',{waitUntil:'domcontentloaded'});
  await page.locator('#import-workshop-xlsx').setInputFiles({
    name:'remito-existing.xlsx',mimeType:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    buffer:syntheticDocument({customer:'Lucía Fernández',phone:'223 551-7820',plate:'AF382QL',make:'Toyota',model:'Corolla',id:'TEST-LAB-EXISTING'})
  });
  await page.locator('#entity-form[data-kind="legacy"]').waitFor({timeout:15000});
  assert.equal(await page.locator('#legacy-client-select').inputValue(),'demo-c1');
  assert.equal(await page.locator('#legacy-vehicle-select').inputValue(),'demo-v1');
  await page.locator('[name="confirmLegacy"]').check();
  await page.locator('#entity-form [type="submit"]').click();
  await page.waitForURL(/#trabajo\//,{timeout:15000});
  const reused=await inspectStore(page);
  assert.equal(reused.clients.length,after.clients.length);
  assert.equal(reused.vehicles.length,after.vehicles.length);
  assert.equal(reused.orders.length,after.orders.length+1);
  assert.deepEqual(errors,[],'No JS console or failed HTTP requests');
  console.log('PASS: real Chrome rendered LAB, bundled XLSX, Settings actions and isolated IndexedDB');
}catch(err){
  const html=(await page.locator('body').innerText().catch(()=>''))?.slice(0,1000);
  console.error('LAB startup failure:',err.message,'BODY:',html,'BROWSER ERRORS:',errors);
  await page.screenshot({path:'/tmp/autotrace-lab-error.png',fullPage:true}).catch(()=>{});
  throw err;
}finally{await browser.close();}
