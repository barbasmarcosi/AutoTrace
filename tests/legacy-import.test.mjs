import test from 'node:test';
import assert from 'node:assert/strict';
import {clientCandidates,vehicleCandidates,suggestedLegacySelection,prepareLegacyImport} from '../site/legacy-import.mjs';
import {validateData} from '../site/model.mjs';
const hash='1234567890abcdef'.repeat(4),today='2026-10-23';
function state(){return validateData({
clients:[{id:'c1',name:'María Pérez',phone:'223 555-1000',email:'maria@test.invalid',notes:'',createdAt:today},
{id:'c2',name:'Juan Fernández',phone:'223 444-0000',email:'',notes:'',createdAt:today}],
vehicles:[{id:'v1',clientId:'c1',plate:'AF123BC',make:'Toyota',model:'Corolla',year:2020,mileage:54000,vin:'ABC123456789XYZ',color:'',nextServiceDate:'',notes:'',createdAt:today},
{id:'v2',clientId:'c2',plate:'AG987WX',make:'Peugeot',model:'208',year:2024,mileage:11000,vin:'',color:'',nextServiceDate:'',notes:'',createdAt:today}],
orders:[],settings:{garageName:'Demo',currency:'ARS',demo:false}
});}
function document(overrides={}){return {documentKind:'work',documentNumber:'S-2026-001',date:today,customerName:'Maria Pérez',
customerPhone:'223-555-1000',customerEmail:'maria@test.invalid',make:'Toyota',model:'Corolla',plate:'AF123BC',
vin:'ABC123456789XYZ',mileage:55500,vehicleObservation:'',sourceFingerprint:hash,total:25000,
items:[{id:'line-20',kind:'part',description:'Filtro',quantity:2,unitPrice:5000},
{id:'line-21',kind:'labor',description:'Diagnóstico',quantity:1,unitPrice:15000}],...overrides};}
let counter=0;const uid=()=>('new-'+(++counter));
function choice(overrides={}){return {clientId:'c1',vehicleId:'v1',confirmed:true,...overrides};}
test('match existing owner using name, phone, email and exact vehicle plate',()=>{
const data=state(),doc=document(),match=suggestedLegacySelection(doc,data);
assert.equal(match.clientId,'c1');assert.equal(match.vehicleId,'v1');
const result=prepareLegacyImport(doc,data,choice(),uid,today);
assert.equal(result.data.clients.length,2);assert.equal(result.data.vehicles.length,2);
assert.equal(result.data.orders.length,1);assert.equal(result.data.vehicles[0].mileage,55500);
assert.equal(data.vehicles[0].mileage,54000);
});
test('create missing client and vehicle in one atomic result',()=>{
const data=state(),doc=document({customerName:'Gabriela Ríos',customerEmail:'gabriela@test.invalid',customerPhone:'1122112211',
plate:'AB555CD',vin:'',make:'Honda',model:'Fit',mileage:300,sourceFingerprint:'a'.repeat(64)});
assert.equal(suggestedLegacySelection(doc,data).clientId,'');
const res=prepareLegacyImport(doc,data,choice({clientId:'new',vehicleId:'new',name:'Gabriela Ríos',email:'gabriela@test.invalid',phone:'1122112211',
plate:'AB555CD',make:'Honda',model:'Fit'}),uid,today);
assert.equal(res.data.clients.length,3);assert.equal(res.data.vehicles.length,3);
assert.equal(res.data.vehicles.at(-1).clientId,res.clientId);assert.equal(res.data.orders[0].vehicleId,res.vehicleId);
assert.equal(data.clients.length,2);
});
test('existing client + new vehicle',()=>{
const doc=document({plate:'AC345DE',vin:'',make:'Ford',model:'Ka'});
const result=prepareLegacyImport(doc,state(),choice({vehicleId:'new',plate:'AC345DE',make:'Ford',model:'Ka'}),uid,today);
assert.equal(result.createdClient,false);assert.equal(result.createdVehicle,true);
});
test('without plate or VIN, model match remains only suggestion',()=>{
const doc=document({plate:'',vin:''});
assert.equal(suggestedLegacySelection(doc,state()).vehicleId,'');
assert.throws(()=>prepareLegacyImport(doc,state(),choice({vehicleId:'new',plate:'',make:'Toyota',model:'Corolla'}),uid,today),/patente/i);
});
test('VIN alone identifies existing car',()=>{
const doc=document({plate:''});
assert.equal(suggestedLegacySelection(doc,state()).vehicleId,'v1');
});
test('no duplicate plate or cross-owner assignment',()=>{
const doc=document({customerName:'Juan Fernández',customerPhone:'2234440000',customerEmail:''});
assert.throws(()=>prepareLegacyImport(doc,state(),choice({clientId:'c2',vehicleId:'v1'}),uid,today),/otro cliente/);
assert.throws(()=>prepareLegacyImport(doc,state(),choice({clientId:'c2',vehicleId:'new',plate:'AF123BC',make:'Toyota',model:'Corolla'}),uid,today),/patente ya existe/);
});
test('never duplicate an existing client with same unique name',()=>{
assert.throws(()=>prepareLegacyImport(document(),state(),choice({clientId:'new',vehicleId:'new',name:'María Pérez',plate:'AR111BC',
make:'Toyota',model:'Corolla'}),uid,today),/Existe un cliente/);
});
test('two names alike are ambiguous without an independent identifier',()=>{
const data=state();data.clients.push({...data.clients[0],id:'c3',phone:'00000111',email:'otro@test.invalid'});
const result=suggestedLegacySelection(document({customerPhone:'',customerEmail:''}),data);
assert.equal(result.clientId,'');assert.equal(clientCandidates(document({customerPhone:'',customerEmail:''}),data.clients).length,2);
});
test('same original file cannot be imported twice',()=>{
const res=prepareLegacyImport(document(),state(),choice(),uid,today);
assert.throws(()=>prepareLegacyImport(document(),res.data,choice(),uid,today),/ya fue importado/);
});
test('same document with different file fingerprint requires explicit duplicate approval',()=>{
const first=prepareLegacyImport(document(),state(),choice(),uid,today);
const next=document({sourceFingerprint:'d'.repeat(64)});
assert.throws(()=>prepareLegacyImport(next,first.data,choice(),uid,today),/parece ya registrado/);
assert.equal(prepareLegacyImport(next,first.data,choice({confirmSimilar:true}),uid,today).data.orders.length,2);
});
test('validation errors keep original data untouched',()=>{
const data=state();const doc=document({plate:'AA123AB',vin:'',customerName:'Nueva Persona'});
const options=choice({clientId:'new',vehicleId:'new',name:'Nueva Persona',plate:'AA123AB',make:'Toyota',model:'Corolla'});
assert.throws(()=>prepareLegacyImport(doc,data,{...options,confirmed:false},uid,today),/Confirmá/);
assert.throws(()=>prepareLegacyImport(doc,data,{...options,model:''},uid,today),/modelo/);
assert.equal(data.orders.length,0);assert.equal(data.clients.length,2);
});
test('search results are capped, and query supports manual lookup',()=>{
const data=state();
for(let i=0;i<35;i++)data.clients.push({...data.clients[0],id:'other-'+i,name:'Nombre '+i,phone:'',email:''});
assert.equal(clientCandidates(document(),data.clients,'Nombre').length,10);
assert.ok(vehicleCandidates(document(),data.vehicles,'c1').length>=1);
});

test('conflicting phone/email prevents automatic identification by name alone',()=>{
  const d=document({customerPhone:'2230000000',customerEmail:'different@test.invalid'});
  const matches=suggestedLegacySelection(d,state());
  assert.equal(matches.clientId,'');
  assert.ok(matches.clients[0].reasons.some(x=>x.includes('diferentes')));
});
test('same-name people can be registered after explicit confirmation, not silently',()=>{
  const d=document({plate:'AS123BC',vin:'',customerPhone:'1122223333',customerEmail:'other@test.invalid'});
  const opts=choice({clientId:'new',vehicleId:'new',name:'Maria Pérez',phone:'1122223333',email:'other@test.invalid',
    plate:'AS123BC',make:'Toyota',model:'Corolla'});
  assert.throws(()=>prepareLegacyImport(d,state(),opts,uid,today),/mismo nombre/);
  const result=prepareLegacyImport(d,state(),{...opts,confirmSameName:true},uid,today);
  assert.equal(result.data.clients.length,3);
});
test('unknown customer fields and unique exact plate infer the registered owner',()=>{
  const doc=document({customerName:'',customerPhone:'',customerEmail:''});
  const hint=suggestedLegacySelection(doc,state());
  assert.equal(hint.clientId,'c1');
  assert.equal(hint.vehicleId,'v1');
});
