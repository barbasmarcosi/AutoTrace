import test from 'node:test';
import assert from 'node:assert/strict';
import {matchesFields,matchesClient,matchesVehicle,matchesOrder,paginate} from '../site/search.mjs';
const c={name:'José Gómez',phone:'223 554-2014',email:'jose@ejemplo.com',notes:'Servicio habitual'};
const v={plate:'AF123BC',make:'Volkswagen',model:'Gol Trend',year:2018,vin:'9BW1234',color:'Azul',notes:''};
const o={title:'Cambio de aceite',category:'Mantenimiento',description:'Filtro de aire',notes:'',status:'progress'};
test('encuentra clientes sin distinguir tildes o mayúsculas',()=>{
 assert.ok(matchesClient(c,'JOSE'));assert.ok(matchesClient(c,'gomez 2014'));
 assert.ok(matchesClient(c,'jose@ejemplo.com'));assert.ok(!matchesClient(c,'roberto'));
});
test('encuentra patentes con o sin espacios y por nombre del propietario',()=>{
 assert.ok(matchesVehicle(v,c,'af 123 bc'));
 assert.ok(matchesVehicle(v,c,'AF-123-BC'));
 assert.ok(matchesVehicle(v,c,'GOL joSe'));
 assert.ok(matchesVehicle(v,c,'2235542014'));
 assert.ok(!matchesVehicle(v,c,'Corolla'));
});
test('busca trabajos por servicio, patente y cliente con múltiples términos',()=>{
 assert.ok(matchesOrder(o,v,c,'aceite jose'));
 assert.ok(matchesOrder(o,v,c,'AF 123 BC'));
 assert.ok(matchesOrder(o,v,c,'mantenimiento gol'));
 assert.ok(!matchesOrder(o,v,c,'bateria'));
});
test('búsqueda vacía encuentra todo, símbolos aislados no producen falsos positivos',()=>{
 assert.ok(matchesFields('',[]));assert.ok(!matchesFields('***',['coche']));
});
test('paginación controla límites y lista vacía',()=>{
 const arr=Array.from({length:28},(_,i)=>i+1);
 assert.deepEqual(paginate(arr,2,12).items,[13,14,15,16,17,18,19,20,21,22,23,24]);
 assert.equal(paginate(arr,999,12).page,3);
 assert.deepEqual(paginate([],1,12),{items:[],total:0,page:1,pages:1,first:0,last:0});
});
