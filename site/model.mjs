/** Funciones puras compartidas por la aplicación y sus pruebas. */
export const normalizePlate = value => String(value ?? '').toUpperCase().replace(/[^A-Z0-9]/g,'');
export const amount = value => {const n=Number(value);return Number.isFinite(n)&&n>=0?Math.round(n*100)/100:0;};
export const pendingAmount = work => Math.max(0,Math.round((amount(work.charged)-amount(work.paid))*100)/100);
export const margin = work => Math.round((amount(work.charged)-amount(work.partsCost)-amount(work.laborCost))*100)/100;
export function csvCell(value){let text=String(value??'');if(/^[\s]*[=+@-]/.test(text))text="'"+text;return `"${text.replace(/"/g,'""')}"`;}
const str=(v,n=3000)=>typeof v==='string'?v.slice(0,n):'';
const number=(value,label)=>{const n=Number(value);if(!Number.isFinite(n)||n<0||n>1e12)throw Error(`El campo ${label} tiene un valor numérico inválido.`);return Math.round(n*100)/100;};
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
  const vehicleIds=new Set(vehicles.map(v=>v.id));
  const validStatuses=new Set(['received','progress','waiting','done','delivered']);
  const orders=data.orders.map(o=>{
    if(!o||typeof o!=='object'||!str(o.id,100).trim()||!vehicleIds.has(o.vehicleId)||!str(o.title,150).trim()||!validStatuses.has(o.status))throw Error('Hay trabajos inválidos.');
    const partsCost=number(o.partsCost,'costo de repuestos'),laborCost=number(o.laborCost,'costo de mano de obra'),charged=number(o.charged,'importe'),paid=number(o.paid,'pagado');
    if(paid>charged)throw Error('Un trabajo tiene un pago mayor a su importe.');
    return {id:str(o.id,100),vehicleId:o.vehicleId,title:str(o.title,150),category:str(o.category,120),status:o.status,openedAt:date(o.openedAt,'de trabajo'),completedAt:date(o.completedAt,'de cierre'),mileage:number(o.mileage,'kilometraje de trabajo'),description:str(o.description),partsCost,laborCost,charged,paid,notes:str(o.notes),createdAt:date(o.createdAt,'de alta de trabajo')};
  });
  checkUnique(orders,'trabajos');
  return {clients,vehicles,orders,settings:{garageName:str(data.settings?.garageName,100)||'Mi taller',currency:['ARS','USD','EUR'].includes(data.settings?.currency)?data.settings.currency:'ARS',demo:!!data.settings?.demo}};
}
