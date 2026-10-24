/** Reversible full XLSX backups shared by browser PWA and desktop Rust v1/v2.
 * No formulas, external links, HTML, workbook macros or implicit dates.
 * This workbook is PRIVATE; it contains all customers and internal costs.
 */
import {validateData} from './model.mjs';
import {sha256} from './browser-excel.mjs';

const CLIENTS=[['id','id'],['nombre','name'],['telefono','phone'],['email','email'],['notas','notes'],['creado_el','createdAt']];
const VEHICLES=[['id','id'],['cliente_id','clientId'],['patente','plate'],['marca','make'],['modelo','model'],['anio','year'],['kilometraje','mileage'],['vin','vin'],['color','color'],['proximo_servicio','nextServiceDate'],['notas','notes'],['creado_el','createdAt']];
const ORDERS_V1=[['id','id'],['vehiculo_id','vehicleId'],['titulo','title'],['categoria','category'],['estado','status'],['fecha_apertura','openedAt'],['fecha_cierre','completedAt'],['kilometraje','mileage'],['descripcion','description'],['costo_repuestos','partsCost'],['costo_mano_obra','laborCost'],['importe_acordado','charged'],['total_pagado','paid'],['notas','notes'],['creado_el','createdAt']];
const ORDERS_V2=[...ORDERS_V1,['tipo_documento','documentKind'],['numero_documento','documentNumber'],['huella_origen','sourceFingerprint']];
const LINES=[['trabajo_id','orderId'],['renglon_id','id'],['tipo','kind'],['descripcion','description'],['cantidad','quantity'],['valor_unitario','unitPrice']];
const SETTINGS=[['nombre_taller','garageName'],['moneda','currency'],['datos_demostracion','demo']];
const ACCOUNTS=[['id','id'],['nombre','name'],['tipo','kind'],['alias','alias']];
const PAYMENTS=[['trabajo_id','orderId'],['cobro_id','id'],['fecha','date'],['importe','amount'],['medio','method'],['cuenta_id','accountId'],['referencia','reference']];
const FLOAT_KEYS=new Set(['mileage','partsCost','laborCost','charged','paid','quantity','unitPrice','amount']);
const numericKeys=new Set([...FLOAT_KEYS]);
const headers=cols=>cols.map(c=>c[0]);
function canonicalEncode(value,key=''){
  if(value===null)return 'null';
  if(typeof value==='boolean')return value?'true':'false';
  if(typeof value==='string')return JSON.stringify(value);
  if(typeof value==='number'){
    if(!Number.isFinite(value))throw Error('Número inválido en respaldo');
    if(FLOAT_KEYS.has(key)&&Number.isInteger(value))return String(value)+'.0';
    return JSON.stringify(value);
  }
  if(Array.isArray(value))return '['+value.map(x=>canonicalEncode(x)).join(',')+']';
  if(value&&typeof value==='object')
    return '{'+Object.keys(value).sort().map(k=>JSON.stringify(k)+':'+canonicalEncode(value[k],k)).join(',')+'}';
  throw Error('Dato incompatible con el respaldo XLSX.');
}
export function canonicalDataJSON(data){
  const state=validateData(data);
  for(const key of ['clients','vehicles','orders'])state[key].sort((a,b)=>a.id<b.id?-1:a.id>b.id?1:0);
  return canonicalEncode(state);
}
async function digestData(data){
  return sha256(new TextEncoder().encode(canonicalDataJSON(data)));
}
function getLib(lib){if(!lib?.utils?.aoa_to_sheet||!lib?.utils?.sheet_to_json)throw Error('Motor Excel no disponible');return lib;}
function emit(book,lib,name,cols,data){
  const rows=[headers(cols),...data.map(x=>cols.map(([h,k])=>x[k]??''))];
  const sheet=lib.utils.aoa_to_sheet(rows);
  sheet['!cols']=cols.map(([h])=>({wch:['descripcion','notas'].includes(h)?40:19}));
  lib.utils.book_append_sheet(book,sheet,name);
}
export async function writeBackupXlsx(data,lib){
  const XLSX=getLib(lib),state=validateData(data),book=XLSX.utils.book_new();
  const lines=state.orders.flatMap(o=>(o.items||[]).map(item=>({orderId:o.id,...item})));
  const accounts=state.settings.accounts||[];
  const payments=state.orders.flatMap(o=>(o.payments||[]).map(payment=>({orderId:o.id,...payment})));
  const meta=[
    ['format','autotrace-backup-xlsx'],['schema_version','3'],
    ['created_at_utc',new Date().toISOString()],['snapshot_id',crypto.randomUUID()],
    ['app_version','0.1.0-pwa'],['source_schema_version','1'],
    ['timezone','America/Argentina/Buenos_Aires'],['export_kind','full'],
    ['canonical_sha256',await digestData(state)],['count_clients',String(state.clients.length)],
    ['count_vehicles',String(state.vehicles.length)],['count_orders',String(state.orders.length)],
    ['count_lines',String(lines.length)],['count_accounts',String(accounts.length)],['count_payments',String(payments.length)]
  ];
  const manifest=XLSX.utils.aoa_to_sheet([['clave','valor'],...meta]);
  XLSX.utils.book_append_sheet(book,manifest,'Manifest');
  emit(book,XLSX,'Clientes',CLIENTS,state.clients);
  emit(book,XLSX,'Vehiculos',VEHICLES,state.vehicles);
  emit(book,XLSX,'Trabajos',ORDERS_V2,state.orders);
  emit(book,XLSX,'DetalleTrabajos',LINES,lines);
  emit(book,XLSX,'Cuentas',ACCOUNTS,accounts);
  emit(book,XLSX,'Cobros',PAYMENTS,payments);
  emit(book,XLSX,'Configuracion',SETTINGS,[state.settings]);
  return new Uint8Array(XLSX.write(book,{type:'array',bookType:'xlsx',compression:true}));
}
function checkNoFormula(sheet,name){
  for(const [addr,c] of Object.entries(sheet)){
    if(addr[0]==='!')continue;
    if(c?.f||c?.l||c?.D)throw Error('Fórmula, vínculo o contenido no permitido en '+name+'!'+addr);
  }
}
function readSheet(book,lib,name,cols,max){
  const ws=book.Sheets[name];if(!ws)throw Error('Falta la hoja '+name);
  checkNoFormula(ws,name);
  const range=lib.utils.decode_range(ws['!ref']||'A1');
  if(range.e.r>max)throw Error('Demasiados registros en '+name);
  const data=lib.utils.sheet_to_json(ws,{header:1,raw:true,defval:null,blankrows:false});
  const expected=headers(cols);
  if(data.length<1||data[0].length!==expected.length||expected.some((h,i)=>data[0][i]!==h))
    throw Error('Encabezados incompatibles en '+name);
  const out=[];
  for(let index=1;index<data.length;index++){
    const row=data[index];
    if(row.length>cols.length&&row.slice(cols.length).some(x=>x!==null&&x!==''))
      throw Error('Columnas desconocidas en '+name);
    if(row.every(x=>x===null||x===''))throw Error('Fila vacía en '+name+': '+(index+1));
    const item={};
    cols.forEach(([,key],column)=>{
      const x=row[column];
      if(key==='demo'){
        if(typeof x!=='boolean')throw Error('La configuración de demostración debe ser booleana.');
        item[key]=x;
      }else if(key==='year'){
        item[key]=(x===null||x==='')?'':x;
        if(item[key]!==''&&(!Number.isInteger(x)||x<1900||x>2200))throw Error('Año inválido');
      }else if(numericKeys.has(key)){
        if(x===null||x==='')item[key]=0;
        else if(typeof x==='number'&&Number.isFinite(x)&&x>=0)item[key]=x;
        else throw Error('Número inválido en '+name+': '+key);
      }else{
        if(x!==null&&x!==''&&typeof x!=='string')throw Error('Celda alterada por Excel en '+name+': '+key);
        item[key]=x??'';
      }
    });
    out.push(item);
  }
  return out;
}
export async function readBackupXlsx(input,lib){
  const XLSX=getLib(lib),bytes=input instanceof Uint8Array?input:new Uint8Array(input);
  if(bytes.byteLength>25_000_000||bytes.byteLength<100)throw Error('Respaldo Excel inválido o demasiado grande.');
  const book=XLSX.read(bytes,{type:'array',cellFormula:true,cellDates:false,cellText:false,bookVBA:false});
  const ws=book.Sheets.Manifest;
  if(!ws)throw Error('No hay hoja Manifest; este archivo no es un respaldo.');
  checkNoFormula(ws,'Manifest');
  const rows=XLSX.utils.sheet_to_json(ws,{header:1,raw:true});
  if(rows[0]?.[0]!=='clave'||rows[0]?.[1]!=='valor')throw Error('Manifest inválido.');
  const meta=new Map();
  for(const [k,v] of rows.slice(1)){
    if(typeof k!=='string'||typeof v!=='string'||meta.has(k))throw Error('Manifest incompleto o repetido.');
    meta.set(k,v);
  }
  const version=meta.get('schema_version');
  if(meta.get('format')!=='autotrace-backup-xlsx'||meta.get('export_kind')!=='full'||!['1','2','3'].includes(version))
    throw Error('El Excel no es un respaldo completo AutoTrace compatible.');
  const required=version==='3'?['Manifest','Clientes','Vehiculos','Trabajos','DetalleTrabajos','Cuentas','Cobros','Configuracion']:
    version==='2'?['Manifest','Clientes','Vehiculos','Trabajos','DetalleTrabajos','Configuracion']:
    ['Manifest','Clientes','Vehiculos','Trabajos','Configuracion'];
  if(book.SheetNames.length!==required.length||required.some(n=>!book.SheetNames.includes(n)))
    throw Error('El respaldo no contiene todas sus hojas.');
  const clients=readSheet(book,XLSX,'Clientes',CLIENTS,15000);
  const vehicles=readSheet(book,XLSX,'Vehiculos',VEHICLES,30000);
  const orders=readSheet(book,XLSX,'Trabajos',version==='1'?ORDERS_V1:ORDERS_V2,100000);
  if(version!=='1'){
    const lines=readSheet(book,XLSX,'DetalleTrabajos',LINES,100000);
    if(meta.get('count_lines')!==String(lines.length))throw Error('Cantidad de renglones incorrecta.');
    const byOrder=new Map(orders.map(o=>[o.id,o]));
    for(const {orderId,...line} of lines){
      const owner=byOrder.get(orderId);if(!owner)throw Error('Renglón sin trabajo asociado.');
      (owner.items??=[]).push(line);
    }
  }
  const settings=readSheet(book,XLSX,'Configuracion',SETTINGS,1);
  if(settings.length!==1)throw Error('Configuración faltante.');
  if(version==='3'){
    const accounts=readSheet(book,XLSX,'Cuentas',ACCOUNTS,100);
    const payments=readSheet(book,XLSX,'Cobros',PAYMENTS,200000);
    if(meta.get('count_accounts')!==String(accounts.length)||meta.get('count_payments')!==String(payments.length))
      throw Error('Cantidad de cuentas/cobros incorrecta.');
    if(accounts.length)settings[0].accounts=accounts;
    const byOrder=new Map(orders.map(o=>[o.id,o]));
    for(const {orderId,...payment} of payments){
      const order=byOrder.get(orderId);if(!order)throw Error('Cobro sin trabajo.');
      (order.payments??=[]).push(payment);
    }
  }
  for(const [k,n] of [['count_clients',clients.length],['count_vehicles',vehicles.length],['count_orders',orders.length]])
    if(meta.get(k)!==String(n))throw Error('Cantidad alterada: '+k);
  const state=validateData({clients,vehicles,orders,settings:settings[0]});
  if(meta.get('canonical_sha256')!==await digestData(state))
    throw Error('El respaldo fue modificado o el checksum no coincide.');
  return state;
}
