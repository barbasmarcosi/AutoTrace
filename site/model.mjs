/** Funciones puras compartidas por la aplicación y sus pruebas. */
export const normalizePlate = value => String(value ?? '').toUpperCase().replace(/[^A-Z0-9]/g,'');
export const amount = value => {const n=Number(value);return Number.isFinite(n)&&n>=0?Math.round(n*100)/100:0;};
export const pendingAmount = work => work.documentKind==='quote'?0:Math.max(0,Math.round((amount(work.charged)-amount(work.paid))*100)/100);
export const margin = work => Math.round((amount(work.charged)-amount(work.partsCost)-amount(work.laborCost))*100)/100;
export const PAYMENT_METHODS = Object.freeze({cash:'Efectivo',transfer:'Transferencia',debit:'Tarjeta de débito',credit:'Tarjeta de crédito',qr:'QR / billetera virtual',other:'Otro'});
export const ELECTRONIC_METHODS = Object.freeze(['transfer','debit','credit','qr']);
export const laborRevenue = work => Math.round((work.items||[]).filter(item=>item.kind==='labor').reduce((total,item)=>total+Math.round(item.quantity*item.unitPrice*100)/100,0)*100)/100;
export const laborContribution = work => (work.items||[]).some(item=>item.kind==='labor') ? Math.round((laborRevenue(work)-amount(work.laborCost))*100)/100 : null;
export const historicalUnallocated = work => Math.round((amount(work.paid)-(work.payments||[]).filter(p=>!p.voidedAt).reduce((total,p)=>total+amount(p.amount),0))*100)/100;
export function csvCell(value){let text=String(value??'');if(/^[\s]*[=+@-]/.test(text))text="'"+text;return `"${text.replace(/"/g,'""')}"`;}
const str=(v,n=3000)=>typeof v==='string'?v.slice(0,n):'';
// Quantities and mileage retain their precision. Money is ALWAYS integer cents
// after validation; silently rounding imported money corrupts historical ledgers.
const number=(value,label)=>{const n=Number(value);if(!Number.isFinite(n)||n<0||n>1e12)throw Error(`El campo ${label} tiene un valor numérico inválido.`);return n;};
const money=(value,label)=>{const n=number(value,label);const cents=n*100;const rounded=Math.round(cents);if(Math.abs(cents-rounded)>0.00001)throw Error(`El campo ${label} tiene más de dos decimales.`);return rounded/100;};
const validDate = s => !s || (/^\d{4}-\d{2}-\d{2}$/.test(s) && !Number.isNaN(new Date(`${s}T12:00:00`).getTime()) && new Date(`${s}T12:00:00`).toISOString().slice(0,10)===s);
const date=(v,label)=>{const s=str(v,10);if(!validDate(s))throw Error(`La fecha ${label} no es válida.`);return s;};
function checkUnique(list,label){if(new Set(list.map(x=>x.id)).size!==list.length)throw Error(`Hay identificadores repetidos en ${label}.`);}
export function validateData(data){
  if(!data||typeof data!=='object'||!Array.isArray(data.clients)||!Array.isArray(data.vehicles)||!Array.isArray(data.orders))throw Error('El archivo no tiene el formato esperado.');
  if(data.clients.length>15000||data.vehicles.length>30000||data.orders.length>100000)throw Error('El respaldo supera los límites admitidos.');
  const clients=data.clients.map(c=>{
    if(!c||typeof c!=='object'||!str(c.id,100).trim()||!str(c.name,120).trim())throw Error('Hay clientes inválidos.');
    return {id:str(c.id,100),name:str(c.name,120),phone:str(c.phone,120),email:str(c.email,120),notes:str(c.notes),createdAt:date(c.createdAt,'de alta de cliente')};
  });
  checkUnique(clients,'clientes'); const clientIds=new Set(clients.map(c=>c.id));
  const vehicles=data.vehicles.map(v=>{
    if(!v||typeof v!=='object'||!str(v.id,100).trim()||!clientIds.has(v.clientId)||!normalizePlate(v.plate)||!str(v.make,120).trim()||!str(v.model,120).trim())throw Error('Hay vehículos inválidos o sin cliente.');
    const plate=normalizePlate(v.plate);if(plate.length<5||plate.length>12)throw Error('Hay patentes inválidas.');
    const year=v.year===''||v.year===null||v.year===undefined?'':number(v.year,'año');
    if(year!==''&&(year<1900||year>new Date().getFullYear()+2||!Number.isInteger(year)))throw Error('Hay años de vehículos inválidos.');
    return {id:str(v.id,100),clientId:v.clientId,plate,make:str(v.make,120),model:str(v.model,120),year,mileage:number(v.mileage,'kilometraje'),vin:str(v.vin,40),color:str(v.color,120),nextServiceDate:date(v.nextServiceDate,'de próximo servicio'),notes:str(v.notes),createdAt:date(v.createdAt,'de alta de vehículo')};
  });
  checkUnique(vehicles,'vehículos');
  if(new Set(vehicles.map(v=>v.plate)).size!==vehicles.length)throw Error('Hay patentes repetidas.');
  const accountsRaw=data.settings?.accounts??[];
  if(!Array.isArray(accountsRaw)||accountsRaw.length>100)throw Error('Cuentas receptoras inválidas.');
  const accounts=accountsRaw.map(a=>{
    if(!a||typeof a!=='object'||!str(a.id,100).trim()||!str(a.name,120).trim()||!['bank','wallet','other'].includes(a.kind))throw Error('Cuenta receptora inválida.');
    return {id:str(a.id,100),name:str(a.name,120),kind:a.kind,alias:str(a.alias,120)};
  });
  checkUnique(accounts,'cuentas');
  const accountIds=new Set(accounts.map(a=>a.id));
  const vehicleIds=new Set(vehicles.map(v=>v.id));
  const validStatuses=new Set(['received','progress','waiting','done','delivered']);
  // Prevent double counting when a document or payment event is assigned to
  // two work orders, including imports and Desktop roundtrips.
  const usedFingerprints=new Set(),usedPaymentIds=new Set();
  const orders=data.orders.map(o=>{
    if(!o||typeof o!=='object')throw Error('Hay trabajos inválidos.');
    const rawLines=o.items===undefined?[]:o.items;
    if(!Array.isArray(rawLines)||rawLines.length>100)throw Error('El detalle del trabajo supera los 100 renglones.');
    const lineIds=new Set();
    const items=rawLines.map(item=>{
      if(!item||typeof item!=='object')throw Error('Renglón de trabajo inválido.');
      const id=str(item.id,100),kind=str(item.kind,20),description=str(item.description,1500);
      if(!id.trim()||!description.trim()||!['part','labor','other'].includes(kind)||lineIds.has(id))throw Error('Hay renglones incompletos o repetidos.');
      lineIds.add(id);
      const quantity=number(item.quantity,'cantidad'),unitPrice=money(item.unitPrice,'valor unitario');
      if(quantity<=0||quantity>100000||quantity*unitPrice>1e12)throw Error('Cantidad o total del renglón fuera de rango.');
      return {id,kind,description,quantity,unitPrice};
    });
    const documentKind=str(o.documentKind,16),documentNumber=str(o.documentNumber,80),sourceFingerprint=str(o.sourceFingerprint,64);
    if(documentKind&&!['quote','work'].includes(documentKind))throw Error('Tipo de documento inválido.');
    if(sourceFingerprint&&!/^[0-9a-f]{64}$/.test(sourceFingerprint))throw Error('Identificador de archivo inválido.');
    if(sourceFingerprint){
      if(usedFingerprints.has(sourceFingerprint))throw Error('Documento de origen duplicado entre trabajos.');
      usedFingerprints.add(sourceFingerprint);
    }
    if(!o||typeof o!=='object'||!str(o.id,100).trim()||!vehicleIds.has(o.vehicleId)||!str(o.title,150).trim()||!validStatuses.has(o.status))throw Error('Hay trabajos inválidos.');
    const partsCost=money(o.partsCost,'costo de repuestos'),laborCost=money(o.laborCost,'costo de mano de obra'),charged=money(o.charged,'importe'),paid=money(o.paid,'pagado');
    if(paid>charged)throw Error('Un trabajo tiene un pago mayor a su importe.');
    const rawPayments=o.payments??[];
    if(!Array.isArray(rawPayments)||rawPayments.length>200)throw Error('Historial de cobros inválido.');
    const payments=rawPayments.map(p=>{
      if(!p||typeof p!=='object'||!str(p.id,100).trim()||!validDate(p.date)||!p.date||!Object.hasOwn(PAYMENT_METHODS,p.method))throw Error('Cobro inválido.');
      if(usedPaymentIds.has(p.id))throw Error('Cobro duplicado entre trabajos; revisá el historial antes de contabilizar.');
      usedPaymentIds.add(p.id);
      const amountPaid=money(p.amount,'cobro');
      if(amountPaid<=0||Math.abs(amountPaid-Number(p.amount))>0.000001)throw Error('Importe de cobro inválido.');
      const accountId=str(p.accountId,100),reference=str(p.reference,160);
      const voidedAt=str(p.voidedAt,10),voidReason=str(p.voidReason,500);
      if(Boolean(voidedAt)!==Boolean(voidReason))throw Error('La anulación requiere fecha y motivo.');
      if(voidedAt&&(!validDate(voidedAt)||voidedAt<p.date||voidReason.trim().length<12))
        throw Error('La fecha o el motivo de anulación no son válidos (mínimo 12 caracteres).');
      if(ELECTRONIC_METHODS.includes(p.method)&&!accountIds.has(accountId))throw Error('El cobro electrónico requiere una cuenta receptora registrada.');
      if(!ELECTRONIC_METHODS.includes(p.method)&&accountId)throw Error('Los cobros no electrónicos no llevan cuenta receptora.');
      return {id:str(p.id,100),date:p.date,amount:amountPaid,method:p.method,accountId,reference,
        ...(voidedAt?{voidedAt,voidReason}:{})};
    });
    checkUnique(payments,'cobros');
    const collected=Math.round(payments.filter(p=>!p.voidedAt).reduce((s,p)=>s+p.amount,0)*100)/100;
    if(collected>paid+0.000001)throw Error('Los cobros detallados superan el total recibido.');
    if(o.documentKind==='quote'&&(paid>0||payments.length))throw Error('Los presupuestos no admiten cobros.');
    return {id:str(o.id,100),vehicleId:o.vehicleId,title:str(o.title,150),category:str(o.category,120),status:o.status,openedAt:date(o.openedAt,'de trabajo'),completedAt:date(o.completedAt,'de cierre'),mileage:number(o.mileage,'kilometraje de trabajo'),description:str(o.description),partsCost,laborCost,charged,paid,notes:str(o.notes),createdAt:date(o.createdAt,'de alta de trabajo'),...(items.length?{items}:{}),...(documentKind?{documentKind}:{}),...(documentNumber?{documentNumber}:{}),...(sourceFingerprint?{sourceFingerprint}:{}),...(payments.length?{payments}:{})};
  });
  checkUnique(orders,'trabajos');
  return {clients,vehicles,orders,settings:{garageName:str(data.settings?.garageName,100)||'Mi taller',currency:['ARS','USD','EUR'].includes(data.settings?.currency)?data.settings.currency:'ARS',demo:!!data.settings?.demo,...(accounts.length?{accounts}:{})}};
}
