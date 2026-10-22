import test from 'node:test';
import assert from 'node:assert/strict';
import {normalizePlate, pendingAmount, margin, csvCell, validateData} from '../site/model.mjs';
const fixture=()=>({clients:[{id:'c1',name:'Lucía'}],vehicles:[{id:'v1',clientId:'c1',plate:'AF 123 BC',make:'Ford',model:'Ka',year:2019,mileage:125000}],orders:[{id:'o1',vehicleId:'v1',title:'Service',status:'progress',openedAt:'2026-10-20',partsCost:100,laborCost:50,charged:250,paid:100,mileage:125000}],settings:{garageName:'Mi taller',currency:'ARS',demo:false}});
test('normalizes plates and calculates pending and margin',()=>{
 assert.equal(normalizePlate('af 123 bc'),'AF123BC');
 const o=fixture().orders[0];assert.equal(pendingAmount(o),150);assert.equal(margin(o),100);
});
test('validates backup and preserves client-vehicle-work relations',()=>{
 const d=validateData(fixture());assert.equal(d.vehicles[0].plate,'AF123BC');assert.equal(d.orders[0].vehicleId,d.vehicles[0].id);assert.equal(d.clients.length,1);
});
test('rejects orphan vehicles and works',()=>{
 let d=fixture();d.vehicles[0].clientId='missing';assert.throws(()=>validateData(d),/vehículos inválidos/);
 d=fixture();d.orders[0].vehicleId='missing';assert.throws(()=>validateData(d),/trabajos inválidos/);
});
test('rejects duplicate plates and ids',()=>{
 let d=fixture();d.vehicles.push({...d.vehicles[0],id:'v2',plate:'AF123BC'});assert.throws(()=>validateData(d),/patentes repetidas/);
 d=fixture();d.clients.push({...d.clients[0]});assert.throws(()=>validateData(d),/identificadores repetidos/);
});
test('rejects invalid amounts, overpayment and dates',()=>{
 let d=fixture();d.orders[0].partsCost=-1;assert.throws(()=>validateData(d),/valor numérico inválido/);
 d=fixture();d.orders[0].paid=999;assert.throws(()=>validateData(d),/pago mayor/);
 d=fixture();d.orders[0].openedAt='2026-02-31';assert.throws(()=>validateData(d),/fecha/);
});
test('guards CSV formula injection and escapes quote',()=>{
 assert.equal(csvCell('=1+2'),'"\'=1+2"');assert.equal(csvCell('A "B"'),'"A ""B"""');
});

test('los desgloses son opcionales para registros viejos y conservan cantidad/importe al validar',()=>{
  const data={
    clients:[{id:'c',name:'Cliente',phone:'',email:'',notes:'',createdAt:''}],
    vehicles:[{id:'v',clientId:'c',plate:'AF012BC',make:'Toyota',model:'Hilux',year:'',mileage:10,vin:'',color:'',nextServiceDate:'',notes:'',createdAt:''}],
    orders:[{id:'o',vehicleId:'v',title:'Presupuesto',category:'Mecánica',status:'received',openedAt:'2026-10-22',completedAt:'',mileage:10,description:'',partsCost:0,laborCost:0,charged:347000,paid:0,notes:'',createdAt:''}],
    settings:{garageName:'Taller',currency:'ARS',demo:false}
  };
  const old=validateData(data);
  assert.equal(old.orders[0].items,undefined);
  data.orders[0].items=[{id:'1',kind:'part',description:'Filtro',quantity:8,unitPrice:19000}];
  data.orders[0].documentKind='quote';
  const renewed=validateData(data);
  assert.equal(renewed.orders[0].items[0].quantity,8);
  assert.equal(pendingAmount(renewed.orders[0]),0);
  data.orders[0].items[0].quantity=-1;
  assert.throws(()=>validateData(data));
});
