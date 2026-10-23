import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {webcrypto} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {parseWorkshopXlsx,writeWorkshopXlsx,writeCustomerReportXlsx} from '../site/browser-excel.mjs';
import {writeBackupXlsx,readBackupXlsx} from '../site/browser-backup.mjs';
import {validateData,pendingAmount} from '../site/model.mjs';

if(!globalThis.crypto?.subtle)Object.defineProperty(globalThis,'crypto',{value:webcrypto,configurable:true});
const require=createRequire(import.meta.url);
const XLSX=require('../site/vendor/xlsx.mini.min.js');
function fixture(){
  return validateData({
    clients:[{id:'c1',name:'Cliente sintético',phone:'001234',email:'cliente@example.test',notes:'PRIVADO_CLIENTE_987',createdAt:'2026-10-20'},
      {id:'c2',name:'Propietario ajeno',phone:'444',email:'',notes:'PRIVADO_AJENO',createdAt:'2026-10-20'}],
    vehicles:[{id:'v1',clientId:'c1',plate:'AF012BC',make:'Toyota',model:'Hilux',year:2018,mileage:125000,vin:'VIN_SECRETO',color:'Rojo',nextServiceDate:'',notes:'NOTA_INTERNA',createdAt:'2026-10-20'},
      {id:'v2',clientId:'c2',plate:'AC123XY',make:'Peugeot',model:'408',year:'',mileage:100,vin:'',color:'',nextServiceDate:'',notes:'',createdAt:'2026-10-20'}],
    orders:[{id:'o1',vehicleId:'v1',title:'Revisión',category:'Mecánica',status:'done',openedAt:'2026-10-20',completedAt:'2026-10-21',
      mileage:125000,description:'Reemplazo',partsCost:7777777,laborCost:8888888,charged:34700,paid:20000,notes:'NOTA_DE_TALLER',
      createdAt:'2026-10-20',documentKind:'work',items:[
        {id:'line1',kind:'part',description:'Filtro de aire',quantity:2,unitPrice:2800},
        {id:'line2',kind:'labor',description:'Mano de obra\ncontrol de bujías',quantity:1,unitPrice:29100}
      ]},
      {id:'o2',vehicleId:'v1',title:'Estimación',category:'Mecánica',status:'received',openedAt:'2026-10-21',completedAt:'',
       mileage:125000,description:'Estimación',partsCost:0,laborCost:0,charged:4000,paid:0,notes:'',createdAt:'2026-10-21',
       documentKind:'quote',items:[{id:'line1',kind:'part',description:'Pieza presupuestada',quantity:1,unitPrice:4000}]},
      {id:'o3',vehicleId:'v2',title:'TRABAJO_AJENO',category:'Mecánica',status:'done',openedAt:'2026-10-20',completedAt:'2026-10-20',
       mileage:100,description:'AJENO',partsCost:0,laborCost:0,charged:123,paid:123,notes:'',createdAt:'2026-10-20'}],
    settings:{garageName:'Taller de prueba',currency:'ARS',demo:false}
  });
}
test('SheetJS offline local version is pinned',()=>{
  assert.equal(XLSX.version,'0.20.3');
});
test('XLSX quote and work documents survive browser export then import',async()=>{
  const data=fixture();
  for(const kind of ['quote','work']){
    const bytes=writeWorkshopXlsx(data,'o1',kind,XLSX);
    const parsed=await parseWorkshopXlsx(bytes,XLSX);
    assert.equal(parsed.documentKind,kind);
    assert.equal(parsed.customerName,'Cliente sintético');
    assert.equal(parsed.plate,'AF012BC');
    assert.equal(parsed.total,34700);
    assert.equal(parsed.items.length,2);
    assert.equal(parsed.items[0].quantity,2);
    assert.match(parsed.items[1].description,/control de bujías/);
  }
});
test('real-style formula caches and description continuation are not counted as new items',async()=>{
  const wb=XLSX.utils.book_new();
  const ws=XLSX.utils.aoa_to_sheet([[]]);
  const set=(id,v)=>ws[id]={t:typeof v==='number'?'n':'s',v};
  set('B1','REMITO TRABAJO');set('J4',46317);set('I6','Cliente de prueba');
  set('D12','Peugeot');set('D13','408');set('D14','AF012BC');set('D16',164438);
  set('B18','REFERENCIA / DETALLE PIEZAS');set('H18','CANTIDAD');set('I18','VALOR UNITARIO');
  set('B20','Foco');set('H20',1);set('I20',4000);set('J20',4000);ws.J20.f='H20*I20';
  set('B21','Mano de obra - control de puesta a punto');set('H21',1);set('I21',9000);set('J21',9000);ws.J21.f='H21*I21';
  set('B22','control de bujías, cambiar taco de motor');set('J22',0);ws.J22.f='H22*I22';
  set('I46','TOTAL');set('J46',13000);ws.J46.f='SUM(J20:J44)';
  ws['!ref']='B1:J46';XLSX.utils.book_append_sheet(wb,ws,'Hoja1');
  const bytes=new Uint8Array(XLSX.write(wb,{type:'array',bookType:'xlsx'}));
  const doc=await parseWorkshopXlsx(bytes,XLSX);
  assert.equal(doc.items.length,2);
  assert.match(doc.items[1].description,/taco de motor/);
  assert.equal(doc.total,13000);
});
test('browser full-backup XLSX v2 roundtrips all IDs, amounts and lines',async()=>{
  const before=fixture();
  const encoded=await writeBackupXlsx(before,XLSX);
  const after=await readBackupXlsx(encoded,XLSX);
  assert.deepEqual(after,before);
  assert.equal(pendingAmount(after.orders[1]),0);
  const wb=XLSX.read(encoded,{type:'array'});
  assert.deepEqual(wb.SheetNames,['Manifest','Clientes','Vehiculos','Trabajos','DetalleTrabajos','Configuracion']);
});
test('tampered workbook is rejected before import',async()=>{
  const bytes=await writeBackupXlsx(fixture(),XLSX);
  const wb=XLSX.read(bytes,{type:'array'});
  wb.Sheets.Trabajos['L2'].v=1;
  const altered=new Uint8Array(XLSX.write(wb,{type:'array',bookType:'xlsx'}));
  await assert.rejects(()=>readBackupXlsx(altered,XLSX),/checksum|inválido|repetido|alterado/i);
});
test('client Excel contains breakdown but never private costs or other client',()=>{
  const bytes=writeCustomerReportXlsx(fixture(),'c1',XLSX);
  const wb=XLSX.read(bytes,{type:'array'});
  assert.deepEqual(wb.SheetNames,['Resumen','Vehículos','Servicios','Detalle de trabajos']);
  const all=wb.SheetNames.flatMap(name=>
    XLSX.utils.sheet_to_json(wb.Sheets[name],{header:1}).flat()
  ).join('|');
  assert.match(all,/Filtro de aire|control de bujías/);
  for(const token of ['PRIVADO_CLIENTE_987','PRIVADO_AJENO','VIN_SECRETO','NOTA_INTERNA','NOTA_DE_TALLER','TRABAJO_AJENO','7777777','8888888','AC123XY']){
    assert.ok(!all.includes(token),'Filtró '+token);
  }
  const services=XLSX.utils.sheet_to_json(wb.Sheets.Servicios,{header:1,defval:''});
  const quote=services.find(row=>row[2]==='Presupuesto');
  assert.equal(quote[7],'');
  assert.equal(quote[8],'');
});
