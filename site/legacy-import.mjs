/** Safe deterministic reconciliation of workshop Excel into AutoTrace.
 * Scoring only suggests; creating or attaching records requires user confirmation.
 * No mutation occurs until the whole batch (client, vehicle, order) passes validation.
 */
import {validateData,normalizePlate} from './model.mjs';
const clean=v=>String(v??'').normalize('NFD').replace(/[\u0300-\u036f]/g,'').toUpperCase().replace(/[^A-Z0-9]+/g,' ').trim().replace(/\s+/g,' ');
const phone=v=>String(v??'').replace(/\D/g,'');
const email=v=>String(v??'').trim().toLowerCase();
const text=v=>String(v??'').trim();
const money=n=>Math.round((Number(n)+Number.EPSILON)*100)/100;
function tokensIn(haystack,needle){
  const words=clean(needle).split(' ').filter(v=>v.length>=3);
  return words.length>=2 && words.every(v=>clean(haystack).split(' ').includes(v));
}
export function clientCandidates(doc,clients,query=''){
  const q=clean(query);
  return clients.map(client=>{
    const docName=clean(doc.customerName),name=clean(client.name);
    const sameEmail=email(doc.customerEmail)&&email(doc.customerEmail)===email(client.email);
    const docPhone=phone(doc.customerPhone),clientPhone=phone(client.phone);
    const samePhone=docPhone.length>=7&&docPhone===clientPhone;
    const sameName=docName.length>=3&&docName===name;
    const similarName=docName.length>=5&&tokensIn(name,docName);
    let score=0,reasons=[];
    if(sameEmail){score+=110;reasons.push('correo');}
    if(samePhone){score+=95;reasons.push('teléfono');}
    if(sameName){score+=70;reasons.push('nombre exacto');}
    else if(similarName){score+=25;reasons.push('nombre parecido');}
    if(q&&!clean([client.name,client.phone,client.email].join(' ')).includes(q))return null;
    if(!q&&score===0)return null;
    return {client,score,reasons,label:reasons.join(' y ')||'búsqueda manual'};
  }).filter(Boolean).sort((a,b)=>b.score-a.score||a.client.name.localeCompare(b.client.name,'es')).slice(0,10);
}
export function vehicleCandidates(doc,vehicles,clientId='',query=''){
  const plate=normalizePlate(doc.plate),vin=clean(doc.vin).replace(/\s/g,'');
  const name=clean([doc.make,doc.model].join(' '));
  const q=clean(query);
  return vehicles.map(vehicle=>{
    const exactPlate=plate.length>=5&&normalizePlate(vehicle.plate)===plate;
    const exactVin=vin.length>=8&&clean(vehicle.vin).replace(/\s/g,'')===vin;
    const sameOwner=!!clientId&&vehicle.clientId===clientId;
    const sameModel=name.length>=4&&name===clean([vehicle.make,vehicle.model].join(' '));
    let score=0,reasons=[];
    if(exactPlate){score+=200;reasons.push('patente exacta');}
    if(exactVin){score+=200;reasons.push('VIN exacto');}
    if(sameOwner){score+=15;reasons.push('mismo cliente');}
    if(sameModel){score+=20;reasons.push('marca y modelo');}
    if(!q&&score<20)return null;
    if(q&&!clean([vehicle.plate,vehicle.make,vehicle.model,vehicle.vin].join(' ')).includes(q))return null;
    return {vehicle,score,exact:exactPlate||exactVin,sameOwner,reason:reasons.join(', ')||'búsqueda manual'};
  }).filter(Boolean).sort((a,b)=>b.score-a.score||a.vehicle.plate.localeCompare(b.vehicle.plate)).slice(0,10);
}
export function suggestedLegacySelection(doc,state){
  const people=clientCandidates(doc,state.clients);
  const winner=people[0];
  // Only auto-select if uniquely identified by exact name, phone or email.
  const clearClient=winner?.score>=70&&(!people[1]||people[1].score<winner.score);
  const clientId=clearClient?winner.client.id:'';
  const cars=vehicleCandidates(doc,state.vehicles,clientId);
  const exact=cars.filter(x=>x.exact&&(!clientId||x.vehicle.clientId===clientId));
  // Auto-select only unique exact plate/VIN, never just same make/model.
  const vehicleId=exact.length===1?exact[0].vehicle.id:'';
  return {clientId,vehicleId,clients:people,vehicles:cars};
}
function required(v,name){const value=text(v);if(!value)throw Error('Completá '+name+'.');return value;}
export function prepareLegacyImport(doc,state,choice,createId=()=>crypto.randomUUID(),today=new Date().toISOString().slice(0,10)){
  if(!doc||!Array.isArray(doc.items)||!['quote','work'].includes(doc.documentKind))throw Error('Documento de taller inválido.');
  if(!/^\d{4}-\d{2}-\d{2}$/.test(doc.date))throw Error('Fecha del documento inválida.');
  if(!choice?.confirmed)throw Error('Confirmá el cliente y el vehículo antes de importar.');
  if(!/^[0-9a-f]{64}$/.test(doc.sourceFingerprint||''))throw Error('No se pudo verificar la identidad del archivo.');
  if(state.orders.some(o=>o.sourceFingerprint===doc.sourceFingerprint))throw Error('Este archivo ya fue importado.');
  const result=structuredClone(state);
  let client;
  if(choice.clientId==='new'){
    const name=required(choice.name,'el nombre del cliente');
    const mail=text(choice.email),telephone=text(choice.phone);
    if(mail&&!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(mail))throw Error('El correo del cliente no es válido.');
    const collisions=clientCandidates({customerName:name,customerPhone:telephone,customerEmail:mail},result.clients);
    if(collisions.some(x=>x.score>=70))throw Error('Existe un cliente con estos datos. Seleccionalo para evitar duplicarlo.');
    client={id:createId(),name,phone:telephone,email:mail,notes:'',createdAt:today};
    result.clients.push(client);
  }else{
    client=result.clients.find(c=>c.id===choice.clientId);
    if(!client)throw Error('El cliente seleccionado no existe.');
  }
  let vehicle;
  if(choice.vehicleId==='new'){
    const plate=normalizePlate(required(choice.plate,'la patente del vehículo'));
    if(plate.length<5||plate.length>12)throw Error('La patente debe tener entre 5 y 12 caracteres.');
    if(doc.plate&&normalizePlate(doc.plate)!==plate)throw Error('La patente ingresada no coincide con el documento.');
    if(result.vehicles.some(v=>normalizePlate(v.plate)===plate))throw Error('La patente ya existe. Elegí el vehículo registrado.');
    const make=required(choice.make,'la marca del vehículo'),model=required(choice.model,'el modelo del vehículo');
    const vin=text(doc.vin);
    if(vin&&result.vehicles.some(v=>v.vin&&clean(v.vin)===clean(vin)))
      throw Error('Ya existe un vehículo con ese VIN. Elegí el vehículo registrado.');
    vehicle={id:createId(),clientId:client.id,plate,make,model,year:'',mileage:doc.mileage||0,vin,color:'',nextServiceDate:'',notes:'',createdAt:today};
    result.vehicles.push(vehicle);
  }else{
    vehicle=result.vehicles.find(v=>v.id===choice.vehicleId);
    if(!vehicle)throw Error('El vehículo seleccionado no existe.');
    if(vehicle.clientId!==client.id)throw Error('El vehículo pertenece a otro cliente. Revisá la asignación antes de importar.');
    if(doc.plate&&normalizePlate(doc.plate)!==normalizePlate(vehicle.plate))
      throw Error('La patente del archivo no coincide con el vehículo seleccionado.');
    if(doc.vin&&vehicle.vin&&clean(doc.vin)!==clean(vehicle.vin))
      throw Error('El VIN del documento no coincide con el vehículo registrado.');
    if(doc.mileage>vehicle.mileage)vehicle.mileage=doc.mileage;
  }
  if(doc.items.length>100)throw Error('El documento tiene demasiados renglones.');
  const items=doc.items.map((item,index)=>({
    id:createId(),kind:item.kind,description:item.description,quantity:item.quantity,unitPrice:item.unitPrice
  }));
  const total=money(items.reduce((acc,i)=>acc+money(i.quantity*i.unitPrice),0));
  if(Math.abs(total-doc.total)>.011)throw Error('El detalle y el total del Excel no coinciden.');
  const similar=result.orders.some(o=>o.vehicleId===vehicle.id&&o.documentKind===doc.documentKind&&
    o.openedAt===doc.date&&Math.abs(o.charged-doc.total)<.011&&
    (doc.documentNumber?o.documentNumber===doc.documentNumber:
      o.items?.length===items.length&&o.items.every((line,index)=>
        clean(line.description)===clean(items[index].description)&&line.quantity===items[index].quantity&&line.unitPrice===items[index].unitPrice)));
  if(similar&&!choice.confirmSimilar)throw Error('Este trabajo parece ya registrado. Confirmá expresamente si querés importarlo otra vez.');
  const id=createId();
  result.orders.push({id,vehicleId:vehicle.id,
    title:doc.documentKind==='quote'?'Presupuesto importado':'Remito de trabajo importado',
    category:doc.documentKind==='quote'?'Presupuesto':'Mecánica',
    status:doc.documentKind==='quote'?'received':'done',
    openedAt:doc.date,completedAt:doc.documentKind==='work'?doc.date:'',
    mileage:doc.mileage||vehicle.mileage,description:doc.vehicleObservation&&!doc.mileage?doc.vehicleObservation:'',
    partsCost:0,laborCost:0,charged:doc.total,paid:0,notes:'',createdAt:today,items,
    documentKind:doc.documentKind,
    ...(doc.documentNumber?{documentNumber:doc.documentNumber}:{}),
    sourceFingerprint:doc.sourceFingerprint});
  return {data:validateData(result),orderId:id,clientId:client.id,vehicleId:vehicle.id,
    createdClient:choice.clientId==='new',createdVehicle:choice.vehicleId==='new',count:items.length};
}
