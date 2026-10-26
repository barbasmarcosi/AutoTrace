/** CI-only independent PDF.js parser: actual output, text extraction, pages. */
import assert from 'node:assert/strict';
import {getDocument} from 'pdfjs-dist/legacy/build/pdf.mjs';
import {writeWorkshopPdf} from '../site/workshop-pdf.mjs';
const lines=Array.from({length:55},(_,i)=>({id:'item-'+i,kind:'labor',
  description:'Revisión eléctrica '+String(i).padStart(2,'0')+' - '+
    'reparación y diagnóstico de sistema de frenos y amortiguación '.repeat(2),
  quantity:1,unitPrice:100.25}));
const charged=Math.round(lines.reduce((sum,i)=>sum+i.unitPrice*100,0))/100;
const data={
  clients:[{id:'c1',name:'María Núñez',phone:'011111111',email:'cliente@example.test',notes:'NUNCA_PUBLICAR_CODIGO_SECRETO',createdAt:'2026-10-25'}],
  vehicles:[{id:'v1',clientId:'c1',plate:'AF012BC',make:'Citroën',model:'C4',year:2020,mileage:80000,vin:'VIN_NO_PUBLICAR',color:'',nextServiceDate:'',notes:'',createdAt:'2026-10-25'}],
  orders:[{id:'o1',vehicleId:'v1',title:'Servicio',category:'Mecánica',status:'done',openedAt:'2026-10-25',
    completedAt:'2026-10-25',mileage:80000,description:'Diagnóstico privado',partsCost:987654,
    laborCost:765432,charged,paid:0,notes:'INTERNO_AISLADO',createdAt:'2026-10-25',items:lines}],
  settings:{garageName:'Taller Peña',currency:'ARS',demo:false}
};
for(const kind of ['quote','work']){
  const bytes=writeWorkshopPdf(data,'o1',kind);
  const loading=getDocument({data:new Uint8Array(bytes),disableFontFace:true,useSystemFonts:true});
  const pdf=await loading.promise;
  assert.ok(pdf.numPages>=2,'Expected multipage A4 '+kind);
  let all='';
  for(let index=1;index<=pdf.numPages;index++){
    const page=await pdf.getPage(index);
    const size=page.getViewport({scale:1});
    assert.ok(Math.abs(size.width-595.28)<.5 && Math.abs(size.height-841.89)<.5);
    const t=await page.getTextContent();
    all+=t.items.map(x=>x.str||'').join(' ')+'\n';
  }
  assert.match(all,/Taller Peña/);
  assert.match(all,/María Núñez/);
  assert.match(all,/Revisión eléctrica 00/);
  assert.match(all,/Revisión eléctrica 54/);
  assert.match(all,/5\.513,75/);
  assert.match(all,kind==='quote'?/PRESUPUESTO/:/REMITO DE TRABAJO/);
  for(const secret of ['NUNCA_PUBLICAR_CODIGO_SECRETO','VIN_NO_PUBLICAR','INTERNO_AISLADO','987654','765432'])
    assert.ok(!all.includes(secret),'Sensitive information appeared in PDF text: '+secret);
  await pdf.destroy();
}
console.log('PASS: PDF.js loaded and extracted real multi-page A4 customer-facing budget/remito PDFs');
