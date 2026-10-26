import test from 'node:test';
import assert from 'node:assert/strict';
import {normalizePlate, pendingAmount, margin, csvCell, validateData, laborRevenue, laborContribution, historicalUnallocated} from '../site/model.mjs';
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

test('mano de obra neta no se inventa cuando el trabajo no tiene renglones',()=>{
  const order=fixture().orders[0];
  assert.equal(laborContribution(order),null);
  order.items=[{id:'labor',kind:'labor',description:'Diagnóstico',quantity:2,unitPrice:250}];
  assert.equal(laborRevenue(order),500);
  assert.equal(laborContribution(order),450);
});
test('cobros electrónicos requieren cuenta y conservan pagos históricos sin clasificar',()=>{
  const data=fixture();
  data.settings.accounts=[{id:'a1',name:'Banco del taller',kind:'bank',alias:'taller.banco'}];
  data.orders[0].payments=[{id:'p1',date:'2026-10-22',amount:25,method:'transfer',accountId:'a1',reference:'OP-001'}];
  let state=validateData(data);
  assert.equal(state.orders[0].payments[0].accountId,'a1');
  assert.equal(historicalUnallocated(state.orders[0]),75);
  data.orders[0].payments[0].accountId='';
  assert.throws(()=>validateData(data),/cuenta receptora/);
  data.orders[0].payments[0].accountId='a1';
  data.orders[0].payments[0].amount=120;
  assert.throws(()=>validateData(data),/superan/);
});
test('efectivo nunca lleva cuenta; cobros y cuentas referenciadas no pueden desaparecer',()=>{
  const data=fixture();
  data.settings.accounts=[{id:'a1',name:'Cuenta principal',kind:'wallet',alias:''}];
  data.orders[0].payments=[{id:'p1',date:'2026-10-22',amount:10,method:'cash',accountId:'a1',reference:''}];
  assert.throws(()=>validateData(data),/no electrónicos/);
  data.orders[0].payments[0].method='qr';
  validateData(data);
  data.settings.accounts=[];
  assert.throws(()=>validateData(data),/cuenta receptora/);
});
test('presupuestos no tienen cobros y el JSON anterior sigue siendo válido',()=>{
  const data=fixture();
  assert.equal(validateData(data).settings.accounts,undefined);
  data.orders[0].documentKind='quote';
  assert.throws(()=>validateData(data),/presupuestos/);
});

test('rejects extra monetary decimals instead of silently rewriting an imported ledger',()=>{
  for(const key of ['charged','paid','partsCost','laborCost']){
    const d=fixture();d.orders[0][key]=key==='paid'?42.001:250.001;
    assert.throws(()=>validateData(d),/más de dos decimales/,key);
  }
  const item=fixture();
  item.orders[0].items=[{id:'line',kind:'part',description:'Repuesto',quantity:1.25,unitPrice:3.333}];
  assert.throws(()=>validateData(item),/más de dos decimales/);
  const payment=fixture();
  payment.orders[0].payments=[{id:'p',date:'2026-10-22',amount:0.001,method:'cash',accountId:'',reference:''}];
  assert.throws(()=>validateData(payment),/más de dos decimales/);
});
test('retains fractional quantities and mileage while preserving exact financial cents',()=>{
  const d=fixture();
  d.vehicles[0].mileage=125000.1234;
  d.orders[0].mileage=125000.1234;
  d.orders[0].items=[{id:'fluid',kind:'part',description:'Aceite fraccionario',quantity:1.2575,unitPrice:33.33}];
  d.orders[0].charged=0.1+0.2+200;
  d.orders[0].paid=0.1+0.2;
  d.orders[0].payments=[{id:'p',date:'2026-10-22',amount:0.1+0.2,method:'cash',accountId:'',reference:''}];
  const validated=validateData(d);
  assert.equal(validated.orders[0].items[0].quantity,1.2575);
  assert.equal(validated.vehicles[0].mileage,125000.1234);
  assert.equal(validated.orders[0].mileage,125000.1234);
  assert.equal(validated.orders[0].charged,200.30);
  assert.equal(validated.orders[0].paid,0.30);
  assert.equal(validated.orders[0].payments[0].amount,0.30);
  assert.equal(historicalUnallocated(validated.orders[0]),0);
});
test('invalid order objects fail closed with meaningful validation errors',()=>{
  const d=fixture(); d.orders[0]=null;
  assert.throws(()=>validateData(d),/trabajos inválidos/);
});

test('prevents double counting across work orders with repeated payment IDs or imported document fingerprints',()=>{
  const data=fixture();
  data.orders.push({...data.orders[0],id:'o2'});
  // Old manual records without a source fingerprint remain valid.
  assert.equal(validateData(data).orders.length,2);
  data.orders[0].sourceFingerprint='a'.repeat(64);
  data.orders[1].sourceFingerprint='a'.repeat(64);
  assert.throws(()=>validateData(data),/Documento de origen duplicado/);
  data.orders[1].sourceFingerprint='b'.repeat(64);
  data.orders[0].payments=[{id:'payment-global',date:'2026-10-22',amount:10,method:'cash',accountId:'',reference:''}];
  data.orders[1].payments=[{...data.orders[0].payments[0]}];
  assert.throws(()=>validateData(data),/Cobro duplicado entre trabajos/);
  data.orders[1].payments[0].id='payment-distinct';
  const valid=validateData(data);
  assert.equal(valid.orders.length,2);
  assert.equal(valid.orders[0].payments[0].id,'payment-global');
  assert.equal(valid.orders[1].payments[0].id,'payment-distinct');
});

test('audited voids retain original receipts and calculate balance in exact cents',()=>{
  const d=fixture();const p={id:'rcpt-1',date:'2026-10-20',amount:80.25,method:'cash',accountId:'',reference:''};
  d.orders[0].paid=100;d.orders[0].payments=[p];
  assert.equal(historicalUnallocated(validateData(d).orders[0]),19.75);
  d.orders[0].paid=19.75;
  d.orders[0].payments=[{...p,voidedAt:'2026-10-25',voidReason:'Se duplicó el ingreso en este trabajo'}];
  const restored=validateData(d);
  assert.equal(restored.orders[0].payments[0].amount,80.25);
  assert.equal(historicalUnallocated(restored.orders[0]),19.75);
  assert.equal(pendingAmount(restored.orders[0]),230.25);
  d.orders[0].payments[0].voidReason='';
  assert.throws(()=>validateData(d),/anulación requiere fecha y motivo/);
  d.orders[0].payments[0].voidReason='Motivo suficiente de anulación';
  d.orders[0].payments[0].voidedAt='2026-10-19';
  assert.throws(()=>validateData(d),/fecha o el motivo/);
});
