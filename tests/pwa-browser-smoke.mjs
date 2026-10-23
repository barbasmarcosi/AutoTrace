/** Browser-level smoke with a real Chrome and persisted IndexedDB. */
import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
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
  assert.deepEqual(errors,[],'No JS console or failed HTTP requests');
  console.log('PASS: real Chrome rendered LAB, bundled XLSX, Settings actions and isolated IndexedDB');
}catch(err){
  const html=(await page.locator('body').innerText().catch(()=>''))?.slice(0,1000);
  console.error('LAB startup failure:',err.message,'BODY:',html,'BROWSER ERRORS:',errors);
  await page.screenshot({path:'/tmp/autotrace-lab-error.png',fullPage:true}).catch(()=>{});
  throw err;
}finally{await browser.close();}
