/**
 * Browser XLSX compatibility layer. The vendor's SheetJS mini build is served
 * locally (same origin), so the PWA can read/write documents while offline.
 * Never evaluate spreadsheet formulas, HTML, hyperlinks or remote resources.
 */
import {validateData} from './model.mjs';

const cell=(ws,addr)=>ws[addr]?.v;
const str=value=>value===null||value===undefined?'':String(value).trim();
const num=(value,label)=>{const n=Number(value);if(!Number.isFinite(n)||n<0||n>1e12)throw Error('Número inválido en '+label);return n;};
const cents=n=>Math.round((n+Number.EPSILON)*100)/100;
const letter='ABCDEFGHIJKLMNOPQRSTUVWXYZ';
const fmtCell=(r,c)=>letter[c]+r;
function getLib(lib){if(!lib||typeof lib.read!=='function'||typeof lib.write!=='function'||!lib.utils)throw Error('El motor Excel no está disponible.');return lib;}
function readWorkbook(input,lib){
  const XLSX=getLib(lib);
  const bytes=input instanceof Uint8Array?input:new Uint8Array(input);
  if(bytes.byteLength>8*1024*1024||bytes.byteLength<100)throw Error('Excel inválido o superior a 8 MB.');
  const book=XLSX.read(bytes,{type:'array',cellFormula:true,cellDates:false,cellText:false,bookVBA:false});
  if(book.SheetNames.length<1||book.SheetNames.length>8)throw Error('Cantidad de hojas Excel no admitida.');
  for(const name of book.SheetNames){
    const ws=book.Sheets[name];
    const range=XLSX.utils.decode_range(ws['!ref']||'A1');
    if(range.e.r>100005||range.e.c>32)throw Error('Hoja Excel demasiado grande.');
  }
  return book;
}
export async function sha256(bytes){
  const data=bytes instanceof Uint8Array?bytes:new Uint8Array(bytes);
  const hash=await crypto.subtle.digest('SHA-256',data);
  return [...new Uint8Array(hash)].map(x=>x.toString(16).padStart(2,'0')).join('');
}
function parseDate(value,lib){
  if(typeof value==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(value))return value;
  if(typeof value!=='number'||!Number.isFinite(value))throw Error('Fecha del Excel inválida.');
  const parts=lib.SSF.parse_date_code(value);
  if(!parts||parts.y<1900||parts.y>2200)throw Error('La fecha del Excel está fuera de rango.');
  return [parts.y,parts.m,parts.d].map((x,i)=>String(x).padStart(i?2:4,'0')).join('-');
}
function assertAllowedLegacyFormulas(ws,lastRow,totalRow){
  for(const [address,entry] of Object.entries(ws)){
    if(address[0]==='!'||!entry||!entry.f)continue;
    const loc=/^([A-Z]+)([0-9]+)$/.exec(address);
    const row=loc?Number(loc[2]):0;
    if(!loc||loc[1]!=='J'||row<20||row>totalRow)
      throw Error('El archivo contiene una fórmula fuera de las columnas permitidas.');
    const formula=String(entry.f).replace(/^\s*=/,'').replace(/\s+/g,'').toUpperCase();
    const allowed=row===totalRow?new RegExp('^SUM\\(J20:J'+lastRow+'\\)$').test(formula):
       formula==='H'+row+'*I'+row;
    if(!allowed||!Number.isFinite(Number(entry.v)))throw Error('Fórmula o resultado del Excel incompatible en '+address+'.');
  }
}
export async function parseWorkshopXlsx(input,lib){
  const XLSX=getLib(lib);const book=readWorkbook(input,XLSX);
  if(book.SheetNames.length!==1)throw Error('El presupuesto o remito debe tener una sola hoja.');
  const ws=book.Sheets[book.SheetNames[0]];
  const title=str(cell(ws,'B1')).toUpperCase();
  const kind=title.includes('PRESUPUESTO')?'quote':title.includes('REMITO TRABAJO')?'work':null;
  if(!kind)throw Error('El Excel no es una plantilla de presupuesto o remito compatible.');
  const last=kind==='quote'?45:44,totalRow=kind==='quote'?47:46;
  if(!str(cell(ws,'B18')).toUpperCase().includes('REFERENCIA')||
     !str(cell(ws,'H18')).toUpperCase().includes('CANTIDAD')||
     !str(cell(ws,'I18')).toUpperCase().includes('UNITARIO')||
     !str(cell(ws,'I'+totalRow)).toUpperCase().includes('TOTAL'))
     throw Error('Los encabezados o el total del documento no coinciden con la plantilla.');
  assertAllowedLegacyFormulas(ws,last,totalRow);
  const items=[];
  for(let row=20;row<=last;row++){
    const description=str(cell(ws,'B'+row));
    const h=cell(ws,'H'+row),i=cell(ws,'I'+row);
    const hasQ=h!==undefined&&h!==null&&h!=='',hasP=i!==undefined&&i!==null&&i!=='';
    if(!description){
      if(hasQ||hasP)throw Error('Fila '+row+': cantidad/precio sin descripción.');
      continue;
    }
    if(!hasQ&&!hasP){
      if(!items.length)throw Error('Fila '+row+': descripción sin renglón anterior.');
      items.at(-1).description+='\n'+description;
      continue;
    }
    if(!hasQ||!hasP)throw Error('Fila '+row+': faltan cantidad o precio.');
    const quantity=num(h,'H'+row),unitPrice=cents(num(i,'I'+row));
    if(!quantity||quantity>100000)throw Error('Cantidad fuera de rango en fila '+row);
    const subtotal=cents(quantity*unitPrice);
    if(Math.abs(num(cell(ws,'J'+row)??0,'J'+row)-subtotal)>.011)throw Error('El subtotal no coincide en fila '+row);
    const upper=description.toUpperCase();
    items.push({id:'line-'+row,kind:upper.includes('MANO DE OBRA')?'labor':'part',description,quantity,unitPrice});
  }
  if(!items.length)throw Error('El archivo no contiene renglones de trabajo.');
  const total=cents(items.reduce((sum,x)=>sum+cents(x.quantity*x.unitPrice),0));
  if(Math.abs(total-num(cell(ws,'J'+totalRow),'total'))>.011)throw Error('El total no coincide con el detalle.');
  const mileageCell=cell(ws,'D16');
  const mileage=typeof mileageCell==='number'?num(mileageCell,'kilometraje'):0;
  const bytes=input instanceof Uint8Array?input:new Uint8Array(input);
  return {
    documentKind:kind,documentNumber:str(cell(ws,'D4')),date:parseDate(cell(ws,'J4'),XLSX),
    workshop:str(cell(ws,'B6')),customerName:str(cell(ws,'I6')),customerPhone:str(cell(ws,'I7')),
    customerAddress:str(cell(ws,'I8')),customerEmail:str(cell(ws,'I9')),customerNotes:str(cell(ws,'I10')),
    make:str(cell(ws,'D12')),model:str(cell(ws,'D13')),plate:str(cell(ws,'D14')),vin:str(cell(ws,'D15')),
    mileage,vehicleObservation:typeof mileageCell==='string'?str(mileageCell):'',
    items,total,sourceFingerprint:await sha256(bytes)
  };
}
function isoToExcelDate(date){
  if(!/^\d{4}-\d{2}-\d{2}$/.test(date))throw Error('El documento no tiene una fecha válida.');
  const value=(Date.parse(date+'T00:00:00Z')-Date.UTC(1899,11,30))/86400000;
  if(!Number.isFinite(value))throw Error('Fecha inválida.');
  return value;
}
const setString=(ws,address,text)=>{ws[address]={t:'s',v:String(text??'')};};
function makeLegacySheet(data,orderId,kind,lib){
  if(!['quote','work'].includes(kind))throw Error('Elegí presupuesto o remito.');
  const state=validateData(data),order=state.orders.find(o=>o.id===orderId);
  if(!order)throw Error('Trabajo inexistente.');
  if(!order.items?.length)throw Error('Cargá el detalle de piezas y tareas antes de exportar.');
  if(order.items.length>26)throw Error('Esta plantilla admite como máximo 26 renglones.');
  const car=state.vehicles.find(v=>v.id===order.vehicleId),client=state.clients.find(c=>c.id===car.clientId);
  const total=cents(order.items.reduce((sum,x)=>sum+cents(x.quantity*x.unitPrice),0));
  if(Math.abs(total-order.charged)>.011)throw Error('El importe acordado y los renglones no coinciden.');
  const ws=lib.utils.aoa_to_sheet([[]]);
  setString(ws,'B1',kind==='quote'?'PRESUPUESTO - ESTIMACION REPARACION':'REMITO TRABAJO');
  setString(ws,'B4',kind==='quote'?'N° Presupuesto:':'N° Remito:');
  setString(ws,'H4',kind==='quote'?'Fecha Creación:':'Fecha Trabajo:');
  if(order.documentNumber)setString(ws,'D4',order.documentNumber);
  ws.J4={t:'n',v:isoToExcelDate(order.openedAt),z:'dd/mm/yyyy'};
  setString(ws,'B6',state.settings.garageName);
  for(const [addr,text] of Object.entries({H6:'Cliente:',I6:client.name,H7:'Teléfono:',I7:client.phone,H8:'Dirección:',H9:'Mail:',I9:client.email,H10:'Observación:',B12:'MARCA:',D12:car.make,B13:'MODELO:',D13:car.model,B14:'MATRICULA:',D14:car.plate,B15:'VIN:',D15:car.vin,B16:'OBSERVACION:'}))setString(ws,addr,text);
  ws.D16={t:'n',v:order.mileage??0};
  for(const [addr,text] of Object.entries({B18:'REFERENCIA / DETALLE PIEZAS',H18:'CANTIDAD',I18:'VALOR UNITARIO',J18:'TOTAL'}))setString(ws,addr,text);
  order.items.forEach((item,index)=>{
    const row=20+index;
    setString(ws,'B'+row,item.description);
    ws['H'+row]={t:'n',v:item.quantity};
    ws['I'+row]={t:'n',v:item.unitPrice};
    ws['J'+row]={t:'n',v:cents(item.quantity*item.unitPrice)};
  });
  const totalRow=kind==='quote'?47:46;
  setString(ws,'I'+totalRow,'TOTAL');ws['J'+totalRow]={t:'n',v:total};
  setString(ws,'B'+totalRow,kind==='quote'?'Presupuesto sujeto a revisión.':'Detalle de trabajo realizado.');
  ws['!ref']='B1:J'+totalRow;
  ws['!cols']=[{wch:3},{wch:42},{wch:14},{wch:14},{wch:14},{wch:14},{wch:12},{wch:14},{wch:20},{wch:18}];
  return ws;
}
export function writeWorkshopXlsx(data,orderId,kind,lib){
  const XLSX=getLib(lib);
  const book=XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(book,makeLegacySheet(data,orderId,kind,XLSX),'Hoja1');
  return new Uint8Array(XLSX.write(book,{type:'array',bookType:'xlsx',compression:true}));
}

/** Customer-facing XLSX, NOT a backup; scoped to exactly one customer.
 * No internal costs, internal notes, VIN, record ids or data from other customers.
 * Estimates are not counted as payments or receivables.
 */
export function writeCustomerReportXlsx(data,clientId,lib){
  const XLSX=getLib(lib),state=validateData(data);
  const client=state.clients.find(c=>c.id===clientId);
  if(!client)throw Error('Cliente inexistente.');
  const vehicles=state.vehicles.filter(v=>v.clientId===clientId);
  const cars=new Map(vehicles.map(v=>[v.id,v]));
  const orders=state.orders.filter(o=>cars.has(o.vehicleId)).sort((a,b)=>b.openedAt.localeCompare(a.openedAt));
  const book=XLSX.utils.book_new();
  const sheet=(name,rows)=>{
    const ws=XLSX.utils.aoa_to_sheet(rows);
    XLSX.utils.book_append_sheet(book,ws,name);
  };
  sheet('Resumen',[
    ['Dato','Valor'],['Taller',state.settings.garageName],['Cliente',client.name],
    ['Teléfono',client.phone],['Correo',client.email],
    ['Documento informativo. No es una factura ni un respaldo.','']
  ]);
  sheet('Vehículos',[
    ['Patente','Vehículo','Año','Kilometraje','Próximo servicio'],
    ...vehicles.map(v=>[v.plate,(v.make+' '+v.model).trim(),v.year,v.mileage,v.nextServiceDate])
  ]);
  sheet('Servicios',[
    ['Fecha','Patente','Documento','Trabajo','Estado','Descripción','Importe presupuestado/acordado','Abonado','Saldo'],
    ...orders.map(o=>{
      const quote=o.documentKind==='quote';
      return [o.openedAt,cars.get(o.vehicleId).plate,quote?'Presupuesto':'Trabajo',o.title,o.status,o.description,
        o.charged,quote?'':o.paid,quote?'':cents(Math.max(0,o.charged-o.paid))];
    })
  ]);
  sheet('Detalle de trabajos',[
    ['Fecha','Patente','Documento','Trabajo','Tipo de renglón','Descripción','Cantidad','Precio unitario','Subtotal'],
    ...orders.flatMap(o=>(o.items||[]).map(item=>[
      o.openedAt,cars.get(o.vehicleId).plate,o.documentKind==='quote'?'Presupuesto':'Trabajo',o.title,
      item.kind==='part'?'Pieza / insumo':item.kind==='labor'?'Mano de obra':'Otro',
      item.description,item.quantity,item.unitPrice,cents(item.quantity*item.unitPrice)
    ]))
  ]);
  return new Uint8Array(XLSX.write(book,{type:'array',bookType:'xlsx',compression:true}));
}
