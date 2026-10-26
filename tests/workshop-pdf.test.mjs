import test from 'node:test';
import assert from 'node:assert/strict';
import {writeWorkshopPdf} from '../site/workshop-pdf.mjs';
import {validateData} from '../site/model.mjs';

function fixture(items=[
  {id:'i1',kind:'part',description:'Filtro de aire (original) - caño 25mm',quantity:2,unitPrice:2500.25},
  {id:'i2',kind:'labor',description:'Revisión de frenos y diagnóstico eléctrico',quantity:1,unitPrice:7500.50}
]){
  const charged=Math.round(items.reduce((t,i)=>t+Math.round(i.quantity*i.unitPrice*100),0))/100;
  return validateData({
    clients:[
      {id:'c1',name:'María José Núñez',phone:'001234',email:'maria@example.test',notes:'OCULTAR_COSTOS_12345',createdAt:'2026-10-25'},
      {id:'c2',name:'CLIENTE_AJENO_SECRETO',phone:'777',email:'',notes:'',createdAt:'2026-10-25'}
    ],
    vehicles:[{id:'v1',clientId:'c1',plate:'AF012BC',make:'Citroën',model:'C4',year:2020,mileage:120000,
      vin:'VIN_PRIVADO_NO_EXPORTAR',color:'',nextServiceDate:'',notes:'',createdAt:'2026-10-25'}],
    orders:[{id:'o1',vehicleId:'v1',title:'Servicio de mantenimiento',category:'Mecánica',status:'done',
      openedAt:'2026-10-25',completedAt:'2026-10-25',mileage:125000,description:'DIAGNOSTICO_INTERNO',
      partsCost:99999.99,laborCost:88888.88,charged,paid:0,notes:'NOTAS_PRIVADAS',
      createdAt:'2026-10-25',documentNumber:'R-0123',items}],
    settings:{garageName:'Taller Peña & Hijos',currency:'ARS',demo:false}
  });
}
function binaryText(bytes){return Buffer.from(bytes).toString('latin1');}
function verifyXref(bytes){
  const str=binaryText(bytes);
  assert.ok(str.startsWith('%PDF-1.4\n'));
  assert.ok(str.trimEnd().endsWith('%%EOF'));
  const match=/startxref\n(\d+)\n%%EOF\n$/.exec(str);
  assert.ok(match,'PDF trailer missing');
  const pos=Number(match[1]);
  assert.ok(str.slice(pos).startsWith('xref\n'));
  const objects=[...str.matchAll(/\n(\d+) 0 obj\n/g)];
  assert.ok(objects.length>5);
  for(const m of objects)assert.equal(Number(m[1])>0,true);
}
test('both offline PDF outputs are real standalone A4 files with scoped line totals',()=>{
  const data=fixture();
  for(const kind of ['quote','work']){
    const bytes=writeWorkshopPdf(data,'o1',kind);
    verifyXref(bytes);
    const pdf=binaryText(bytes);
    assert.match(pdf,/MediaBox \[0 0 595\.28 841\.89\]/);
    assert.match(pdf,kind==='quote'?/PRESUPUESTO/:/REMITO DE TRABAJO/);
    assert.match(pdf,/Taller Pe\xf1a & Hijos/);
    assert.match(pdf,/Mar\xeda Jos\xe9 N\xfa\xf1ez/);
    assert.match(pdf,/Filtro de aire/);
    assert.match(pdf,/TOTAL/);
    for(const secret of ['OCULTAR_COSTOS_12345','CLIENTE_AJENO_SECRETO',
      'VIN_PRIVADO_NO_EXPORTAR','DIAGNOSTICO_INTERNO','NOTAS_PRIVADAS','$ 99.999,99','$ 88.888,88']){
      assert.ok(!pdf.includes(secret),'Privacy leak: '+secret);
    }
    assert.ok(bytes.length<100000,'No network, font bundles or embedded internal data');
  }
});
test('long 100-row documents paginate and preserve all item content without truncation',()=>{
  const items=Array.from({length:100},(_,i)=>({id:'line'+i,kind:'labor',
    description:'Tarea '+String(i).padStart(3,'0')+' - '+
      ('revisión mecánica de frenos y amortiguación, verificación de piezas; '.repeat(5)),
    quantity:1,unitPrice:1.25}));
  const pdf=binaryText(writeWorkshopPdf(fixture(items),'o1','work'));
  const pages=[...pdf.matchAll(/\/Type \/Page \/Parent/g)].length;
  assert.ok(pages>=4,'Rows did not paginate');
  assert.match(pdf,/Tarea 000/);
  assert.match(pdf,/Tarea 099/);
  assert.match(pdf,new RegExp('P.gina 1 de '+pages)); // WinAnsi í in binary
  verifyXref(Buffer.from(pdf,'latin1'));
});
test('fail closed on missing items, inconsistent totals or unsupported Unicode',()=>{
  const empty=fixture();empty.orders[0].items=[];
  assert.throws(()=>writeWorkshopPdf(empty,'o1','quote'),/detalle/);
  const invalid=fixture();
  invalid.orders[0].charged=1;
  assert.throws(()=>writeWorkshopPdf(invalid,'o1','work'),/coincid|renglones|Detalle/i);
  assert.throws(()=>writeWorkshopPdf(fixture(),'unknown','quote'),/inexistente/);
  assert.throws(()=>writeWorkshopPdf(fixture(),'o1','fiscal'),/presupuesto o remito/);
  const unsupported=fixture();
  unsupported.clients[0].name='Alice 🛠️';
  assert.throws(()=>writeWorkshopPdf(unsupported,'o1','quote'),/carácter no compatible/);
});
test('PDF string escaping neutralizes parentheses, backslashes and injected markup',()=>{
  const data=fixture([{id:'i1',kind:'labor',quantity:1,unitPrice:50,
    description:'Cierre ) Tj ET \\ posible (texto) <script>alert(1)</script>'}]);
  const pdf=binaryText(writeWorkshopPdf(data,'o1','work'));
  assert.match(pdf,/Cierre \\[)] Tj ET/);
  assert.match(pdf,/\\\(/);
  assert.match(pdf,/\\\\ posible/);
  verifyXref(Buffer.from(pdf,'latin1'));
});
