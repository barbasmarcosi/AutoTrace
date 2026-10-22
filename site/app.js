import {validateData, csvCell, pendingAmount, margin} from "./model.mjs";
import {matchesClient, matchesVehicle, matchesOrder, paginate, filterVehicleHistory} from "./search.mjs";
import {buildCustomerSummary} from "./customer-summary.mjs";
import {suggestVehicles, suggestClients} from "./pickers.mjs";
/** AutoTrace v1 — sin backend, datos almacenados exclusivamente en este navegador. */
const DB_NAME = 'autotrace-v1';
const STORE = 'application';
const KEY = 'records';
const STATUSES = { received: 'Recibido', progress: 'En proceso', waiting: 'Esperando repuesto', done: 'Finalizado', delivered: 'Entregado' };
const STATUS_STYLE = { received: 'received', progress: 'progress', waiting: 'waiting', done: 'done', delivered: 'delivered' };
const CATEGORIES = ['Diagnóstico', 'Mantenimiento', 'Mecánica', 'Electricidad', 'Electrónica', 'Neumáticos', 'Otro'];
const NAV = [{id:'inicio',label:'Inicio',icon:'home'}, {id:'clientes',label:'Clientes',icon:'users'}, {id:'vehiculos',label:'Vehículos',icon:'car'}, {id:'trabajos',label:'Trabajos',icon:'clipboard'}, {id:'finanzas',label:'Finanzas',icon:'wallet'}, {id:'ajustes',label:'Ajustes',icon:'settings'}];
const PATHS = {
 home:'<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z"/>',
 users:'<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/>',
 car:'<path d="M5 17h14M5 17l-1 2v2m15-4 1 2v2M5 17H3v-5l3-7h12l3 7v5h-2M7 21h.01M17 21h.01M3 12h18M7.5 8.5h.01M16.5 8.5h.01"/>',
 clipboard:'<rect x="5" y="4" width="14" height="18" rx="2"/><path d="M9 4.5h6V2H9zM9 10h6M9 14h6M9 18h4"/>',
 wallet:'<rect x="2" y="5" width="20" height="16" rx="2"/><path d="M2 9h20M16 14h3"/>',
 settings:'<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.8 1.8 0 0 0 .35 2l-2 2a1.8 1.8 0 0 0-2-.35 8 8 0 0 1-1.6.67 1.8 1.8 0 0 0-1.15 1.45h-2a1.8 1.8 0 0 0-1.15-1.45 8 8 0 0 1-1.6-.67 1.8 1.8 0 0 0-2 .35l-2-2a1.8 1.8 0 0 0 .35-2 8 8 0 0 1-.67-1.6A1.8 1.8 0 0 0 2.5 12v-1a1.8 1.8 0 0 0 1.45-1.15 8 8 0 0 1 .67-1.6 1.8 1.8 0 0 0-.35-2l2-2a1.8 1.8 0 0 0 2 .35 8 8 0 0 1 1.6-.67A1.8 1.8 0 0 0 11 2.5h2a1.8 1.8 0 0 0 1.15 1.45 8 8 0 0 1 1.6.67 1.8 1.8 0 0 0 2-.35l2 2a1.8 1.8 0 0 0-.35 2 8 8 0 0 1 .67 1.6A1.8 1.8 0 0 0 21.5 11v1a1.8 1.8 0 0 0-1.45 1.15 8 8 0 0 1-.65 1.85z"/>',
 search:'<circle cx="11" cy="11" r="7"/><path d="m16 16 5 5"/>',
 plus:'<path d="M12 5v14M5 12h14"/>',
 arrow:'<path d="M5 12h14m-6-6 6 6-6 6"/>',
 back:'<path d="M19 12H5m6 6-6-6 6-6"/>',
 chevron:'<path d="m9 18 6-6-6-6"/>',
 x:'<path d="M18 6 6 18M6 6l12 12"/>',
 check:'<path d="m5 12 4 4L19 6"/>',
 clock:'<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
 calendar:'<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 3v4M17 3v4M3 10h18"/>',
 trending:'<path d="m3 17 6-6 4 4 8-8M16 7h5v5"/>',
 activity:'<path d="M3 12h4l3-8 4 16 3-8h4"/>',
 phone:'<path d="M22 16.92v3a2 2 0 0 1-2.18 2A19.8 19.8 0 0 1 3.09 5.18 2 2 0 0 1 5.08 3h3a2 2 0 0 1 2 1.72l.42 2.8a2 2 0 0 1-.57 1.76L8 11a16 16 0 0 0 5 5l1.72-1.93a2 2 0 0 1 1.76-.57l2.8.42a2 2 0 0 1 1.72 2z"/>',
 mail:'<rect x="2" y="4" width="20" height="16" rx="2"/><path d="m2 6 10 7L22 6"/>',
 gauge:'<path d="M4.8 17.5a9 9 0 1 1 14.4 0M12 13l4-4M9 20h6"/>',
 download:'<path d="M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4"/>',
 upload:'<path d="M12 17V5m-5 5 5-5 5 5M5 17v4h14v-4"/>',
 trash:'<path d="M3 6h18M8 6V4h8v2m3 0-1 15H6L5 6M10 10v7M14 10v7"/>',
 pencil:'<path d="m15 5 4 4M4 20l4.5-1 11-11a2 2 0 0 0-4-4l-11 11L4 20z"/>',
 alert:'<path d="m12 3 10 18H2L12 3zM12 10v4M12 17h.01"/>',
 shield:'<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="m9 12 2 2 4-4"/>',
 wrench:'<path d="M14.5 6.5a5 5 0 0 0-6 6L3 18l3 3 5.5-5.5a5 5 0 0 0 6-6L14 13l-3-3z"/>',
 menu:'<path d="M4 7h16M4 12h16M4 17h16"/>',
 database:'<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 4 18 4 18 0V5M3 12c0 4 18 4 18 0"/>',
 sparkle:'<path d="m12 3 1.6 5.4L19 10l-5.4 1.6L12 17l-1.6-5.4L5 10l5.4-1.6zM19 17l1 2 2 1-2 1-1 2-1-2-2-1 2-1z"/>',
 printer:'<path d="M6 9V3h12v6M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2M6 15h12v6H6z"/>',
 info:'<circle cx="12" cy="12" r="10"/><path d="M12 11v6M12 7h.01"/>',
 tag:'<path d="M20 13 13 20a2 2 0 0 1-3 0l-7-7V4h9l8 8a2 2 0 0 1 0 1z"/><circle cx="7.5" cy="8.5" r="1"/>',
 chart:'<path d="M3 3v18h18M7 16v-4m5 4V7m5 9V4"/>'
};
const icon = (name, size=18) => `<svg aria-hidden="true" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round">${PATHS[name] || PATHS.info}</svg>`;
const esc = (value) => String(value ?? '').replace(/[&<>"']/g, char => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const isoDate = (d) => { const date = new Date(d); return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`; };
const dateShift = (days) => { const d = new Date(); d.setHours(12,0,0,0); d.setDate(d.getDate()+days); return isoDate(d); };
const formatDate = (input) => input && /^\d{4}-\d{2}-\d{2}$/.test(input) ? new Date(`${input}T12:00:00`).toLocaleDateString('es-AR',{day:'2-digit',month:'short',year:'numeric'}) : '—';
const compactDate = (input) => input ? new Date(`${input}T12:00:00`).toLocaleDateString('es-AR',{day:'2-digit',month:'short'}) : '—';
const money = value => new Intl.NumberFormat('es-AR',{style:'currency',currency:db.settings.currency,maximumFractionDigits:0}).format(Number(value)||0);
const num = value => new Intl.NumberFormat('es-AR').format(Number(value)||0);
const uid = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
const sum = (arr, fn) => arr.reduce((acc,v) => acc + Number(fn(v)||0),0);
const amount = value => { const n = Number(value); return Number.isFinite(n) && n >= 0 ? Math.round(n*100)/100 : 0; };
const normPlate = v => String(v||'').normalize('NFD').replace(/[\u0300-\u036f]/g,'').toUpperCase().replace(/[^A-Z0-9]/g,'');
const initials = s => String(s||'').trim().split(/\s+/).slice(0,2).map(x=>x[0]).join('').toUpperCase();
const daysUntil = str => Math.round((new Date(`${str}T12:00:00`)-new Date(`${isoDate(new Date())}T12:00:00`))/86400000);
const canonical = () => ({clients:[],vehicles:[],orders:[],settings:{garageName:'Mi taller',currency:'ARS',demo:false}});
function seedDemo(){
  const data=canonical(); data.settings.demo=true;
  const people=[['Lucía Fernández','223 551-7820','lucia@example.com'],['Martín López','223 618-0942','martin@example.com'],['Sofía Romero','223 599-4206',''],['Diego Martínez','223 426-1188',''],['Paula Gómez','223 681-2025','paula@example.com']];
  data.clients=people.map((p,i)=>({id:`demo-c${i+1}`,name:p[0],phone:p[1],email:p[2],notes:'',createdAt:dateShift(-45+i*3)}));
  const cars=[['demo-c1','AF 382 QL','Toyota','Corolla',2021,64500,22],['demo-c2','AB 925 TY','Volkswagen','Golf',2018,104200,65],['demo-c3','AE 148 NM','Ford','Ranger',2020,88200,9],['demo-c4','AC 687 JP','Chevrolet','Onix',2019,76850,-6],['demo-c5','AG 412 BK','Peugeot','208',2023,29100,90],['demo-c1','AD 592 MS','Honda','HR-V',2020,53100,18]];
  data.vehicles=cars.map((v,i)=>({id:`demo-v${i+1}`,clientId:v[0],plate:normPlate(v[1]),make:v[2],model:v[3],year:v[4],mileage:v[5],vin:'',color:'',nextServiceDate:dateShift(v[6]),notes:'',createdAt:dateShift(-45)}));
  const works=[['demo-v1','Service de 60.000 km','Mantenimiento','progress',-2,45000,30000,113000,40000],['demo-v2','Diagnóstico de inyección','Diagnóstico','waiting',-4,10000,35000,84000,0],['demo-v3','Revisión de frenos delanteros','Mecánica','received',0,35000,28000,97000,0],['demo-v4','Cambio de batería','Electricidad','done',-8,88000,15000,136000,136000],['demo-v5','Revisión electrónica preventiva','Electrónica','delivered',-12,0,42000,65000,65000],['demo-v6','Cambio de aceite y filtros','Mantenimiento','delivered',-20,51000,19000,104000,104000],['demo-v2','Distribución completa','Mecánica','delivered',-38,130000,75000,279000,279000],['demo-v1','Escaneo computarizado','Diagnóstico','delivered',-52,0,24000,38000,38000]];
  data.orders=works.map((w,i)=>({id:`demo-o${i+1}`,vehicleId:w[0],title:w[1],category:w[2],status:w[3],openedAt:dateShift(w[4]),completedAt:['done','delivered'].includes(w[3])?dateShift(w[4]+1):'',mileage:data.vehicles.find(v=>v.id===w[0]).mileage,description:i===0?'Mantenimiento periódico, revisión general de fluidos y tren delantero.':i===1?'Verificar presión de combustible y revisar sensores.':'',partsCost:w[5],laborCost:w[6],charged:w[7],paid:w[8],notes:'',createdAt:dateShift(w[4])}));
  return data;
}
let db = canonical();
let activeQuery = '';
let activeFilter = 'all';
const listQueries = { clientes: '', vehiculos: '', trabajos: '' };
const listPages = { clientes: 1, vehiculos: 1, trabajos: 1 };
let financeQuery = '';
let financeFilter = 'all';
let financePage = 1;
let modalOriginal = '';
let mobileSearchOpen = false;
const modalSnapshot = form => JSON.stringify([...new FormData(form).entries()]);
let clientFilter = 'all';
let vehicleFilter = 'all';
let historyVehicle = '';
let historyQuery = '';
let historyStatus = 'all';
let historyPage = 1;
let dbConnection = null;
const desktopInvoke = window.__TAURI__?.core?.invoke ?? null;
let desktopRevision = 0;
let lastDesktopBackup = null;
let desktopBackupBusy = false;
let memoryOnly = false;
let toastTimeout;
const app = document.querySelector('#app');
function openDatabase(){
  return new Promise((resolve,reject)=>{
    const req=indexedDB.open(DB_NAME,1);
    req.onupgradeneeded=()=>{ if(!req.result.objectStoreNames.contains(STORE))req.result.createObjectStore(STORE); };
    req.onsuccess=()=>resolve(req.result); req.onerror=()=>reject(req.error);
  });
}
async function loadData(){
  if(desktopInvoke){
    // El escritorio nunca siembra registros ficticios ni reutiliza IndexedDB por error.
    // Fallar cerrado ante daños o incompatibilidades: no permitir ediciones en memoria.
    const snapshot=await desktopInvoke('load_state');
    db=validateData(snapshot.data);
    desktopRevision=snapshot.revision;
    return;
  }
  try {
    dbConnection=await openDatabase();
    const saved=await new Promise((resolve,reject)=>{const tx=dbConnection.transaction(STORE,'readonly');const req=tx.objectStore(STORE).get(KEY);req.onsuccess=()=>resolve(req.result);req.onerror=()=>reject(req.error);});
    db=saved?validateData(saved):seedDemo();
    if(!saved)await saveData(db);
  } catch(err){ console.error('AutoTrace storage unavailable:',err); memoryOnly=true;db=seedDemo(); }
}
async function saveData(next){
  if(desktopInvoke){
    // La revisión SQL protege contra una segunda ventana con datos desactualizados.
    const canonical=validateData(next);
    const result=await desktopInvoke('save_state',{data:canonical,expectedRevision:desktopRevision});
    desktopRevision=result.revision;
    db=canonical;
    return;
  }
  if(memoryOnly)throw Error('El almacenamiento local no está disponible. No se guardaron los cambios.');
  await new Promise((resolve,reject)=>{
    const tx=dbConnection.transaction(STORE,'readwrite');tx.objectStore(STORE).put(next,KEY);tx.oncomplete=resolve;tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error);
  });
  db=next;
}
async function updateData(mutator, message){
  try {
    const next=structuredClone(db); mutator(next); await saveData(next); render(); if(message)toast(message);
    return true;
  }catch(err){console.error(err);toast(err?.message||'No se pudieron guardar los cambios','error');return false;}
}
function toast(message,type='success'){
  const region=document.querySelector('#toast-region');const el=document.createElement('div');el.className=`toast ${type==='error'?'error':''}`;el.textContent=message;region.appendChild(el);clearTimeout(toastTimeout);toastTimeout=setTimeout(()=>{el.remove();},4500);if(region.children.length>3)region.firstElementChild.remove();
}
const findClient=id=>db.clients.find(c=>c.id===id);
const findVehicle=id=>db.vehicles.find(v=>v.id===id);
const findOrder=id=>db.orders.find(o=>o.id===id);
const fullVehicle=v=>v?`${v.make} ${v.model}`:'Vehículo eliminado';
const vehicleClient=v=>v?findClient(v.clientId):null;
const activeOrders=()=>db.orders.filter(o=>['received','progress','waiting'].includes(o.status));
const route=()=>{const h=decodeURIComponent(location.hash.replace(/^#\/?/,''));const [page,id]=h.split('/');return {page:page||'inicio',id:id||''};};
function navigate(page,id='') {activeQuery='';mobileSearchOpen=false;location.hash=`#${page}${id?`/${encodeURIComponent(id)}`:''}`;if(route().page===page)render();window.scrollTo({top:0,behavior:'instant'});}
const navActive = page => ({cliente:'clientes',vehiculo:'vehiculos',trabajo:'trabajos',buscar:'inicio'}[page]||page);
const button=(label,action,style='btn-secondary',attrs='')=>`<button type="button" class="btn ${style}" data-action="${action}" ${attrs}>${label}</button>`;
const pill=status=>`<span class="pill pill-${STATUS_STYLE[status]||'received'}">${esc(STATUSES[status]||status)}</span>`;
const sectionHead=(title,subtitle,actions='')=>`<div class="heading"><div><p class="eyebrow">AUTOTRACE / ${esc(title.toUpperCase())}</p><h1>${esc(title)}</h1><p>${esc(subtitle)}</p></div>${actions?`<div class="heading-actions">${actions}</div>`:''}</div>`;
const empty=(title,description,action='',symbol='clipboard')=>`<div class="empty"><div class="empty-illustration">${icon(symbol,28)}</div><h3>${esc(title)}</h3><p>${esc(description)}</p>${action}</div>`;
const phoneLink = value => value ? `<a class="contact-link" href="tel:${esc(String(value).replace(/[^+\d]/g,''))}">${icon('phone',14)} ${esc(value)}</a>` : '—';
const descView=(label,value)=>`<div class="info-item"><small>${esc(label)}</small><strong>${value||'—'}</strong></div>`;
function render(){
  const current=route(); const currentNav=navActive(current.page);
  app.innerHTML=`<a href="#page-content" class="skip-link">Saltar al contenido</a><div class="shell"><aside class="sidebar"><a class="brand" href="#inicio" aria-label="AutoTrace, ir al inicio"><span class="brand-logo">${icon('car',24)}</span><span><span class="brand-name">AutoTrace</span><span class="brand-sub">Gestión de taller</span></span></a><div class="sidebar-label">ESPACIO DE TRABAJO</div><nav aria-label="Navegación principal">${NAV.map(n=>`<button class="nav-item ${currentNav===n.id?'active':''}" data-nav="${n.id}" ${currentNav===n.id?'aria-current="page"':''}>${icon(n.icon)}${n.label}${n.id==='trabajos'&&activeOrders().length?`<span class="count">${activeOrders().length}</span>`:''}</button>`).join('')}</nav><div class="sidebar-spacer"></div><div class="sidebar-footer"><div class="online"><span class="status-dot"></span>Guardado en este dispositivo</div><p>Funciona sin conexión. Hacé respaldos frecuentes: los datos no se sincronizan entre dispositivos.</p></div></aside><div class="main-column"><header class="topbar ${mobileSearchOpen?'search-open':''}"><div class="topbar-title">Tu taller <span class="slash">/</span> <span class="desktop-only">${esc(NAV.find(n=>n.id===currentNav)?.label||'Detalle')}</span></div><div class="topbar-right"><button type="button" class="mobile-search-toggle" data-action="open-mobile-search" aria-label="Abrir búsqueda global">${icon('search',21)}</button><div class="search-wrap">${icon('search',17)}<input class="search" id="global-search" type="search" aria-label="Buscar en AutoTrace" placeholder="Buscar en todo el taller…" title="Atajo: / o Ctrl + K" value="${esc(activeQuery)}" autocomplete="off" />${activeQuery?`<button class="search-clear" data-action="clear-search" aria-label="Borrar búsqueda">${icon('x',16)}</button>`:''}</div><button type="button" class="mobile-search-close" data-action="close-mobile-search" aria-label="Cerrar búsqueda">${icon('x',20)}</button>${button(`${icon('plus')}<span class="topbar-create-label">Nuevo trabajo</span>`,'new-order','btn-primary topbar-create','aria-label="Nuevo trabajo"')}${button(icon('settings'),'go-settings','btn-secondary icon-btn','aria-label="Ajustes"')}</div></header><main class="content" id="page-content" tabindex="-1">${memoryOnly?`<div class="notice warning storage-alert" role="alert">${icon('alert',17)} No se puede guardar en este navegador. No registres datos reales en esta sesión.</div>`:''}${renderPage()}</main></div></div><nav class="mobile-nav" aria-label="Navegación móvil">${NAV.filter(x=>x.id!=='ajustes').map(n=>`<button class="${currentNav===n.id?'active':''}" data-nav="${n.id}" aria-label="${n.label}" ${currentNav===n.id?'aria-current="page"':''}>${icon(n.icon,21)}<span>${n.label}</span></button>`).join('')}</nav>`;
}
function renderPage(){
  if(activeQuery.trim())return renderSearch();
  const r=route();
  switch(r.page){case 'inicio':return renderDashboard();case 'clientes':return renderClients();case 'vehiculos':return renderVehicles();case 'trabajos':return renderOrders();case 'finanzas':return renderFinance();case 'ajustes':return renderSettings();case 'cliente':return renderClientDetail(r.id);case 'vehiculo':return renderVehicleDetail(r.id);case 'trabajo':return renderOrderDetail(r.id);default:return empty('Página no encontrada','Elegí una sección para continuar.',button('Volver al inicio','go-home'));}
}
function refreshPage(){const target=document.querySelector('#page-content');if(target)target.innerHTML=renderPage();}
function demoBanner(){return db.settings.demo?`<div class="demo-banner"><p><strong>Modo demostración.</strong><span class="demo-explanation"> Los nombres, vehículos e importes son ficticios. Podés explorar sin riesgo y luego comenzar con tus propios registros.</span><span class="demo-mini"> Datos ficticios para explorar.</span></p>${button('Comenzar desde cero','reset-demo','btn-secondary btn-sm')}</div>`:'';}
function renderDashboard(){
  const recent=[...db.orders].sort((a,b)=>b.openedAt.localeCompare(a.openedAt)).slice(0,5);
  const open=activeOrders();
  const thisMonth=isoDate(new Date()).slice(0,7);
  const monthly=db.orders.filter(x=>x.openedAt.startsWith(thisMonth));
  const due=db.vehicles.filter(v=>v.nextServiceDate&&daysUntil(v.nextServiceDate)<=30).sort((a,b)=>a.nextServiceDate.localeCompare(b.nextServiceDate)).slice(0,5);
  const pending=sum(db.orders,pendingAmount);
  return `${demoBanner()}<div class="heading"><div><p class="eyebrow">PANEL DE CONTROL</p><h1>${db.settings.garageName==='Mi taller'?'Tu taller, hoy':`Resumen de ${esc(db.settings.garageName)}`}</h1><p>Todo lo que necesitás resolver, en un solo lugar.</p></div><span class="badge-local">${icon('shield',15)} Privado y local</span></div>
  <section class="hero"><div class="hero-content"><div class="hero-eyebrow">${icon('sparkle',16)} ACCIONES RÁPIDAS</div><h2>Todo listo para tu próximo trabajo.</h2><p>Creá una orden o sumá un cliente sin dar vueltas.</p><div class="hero-actions">${button(`${icon('plus')} Registrar trabajo`,'new-order','btn-lime')}${button(`${icon('users')} Nuevo cliente`,'new-client','btn-secondary')}</div></div><div class="hero-watermark">${icon('car',210)}</div></section>
  <div class="priority-strip" aria-label="Pendientes del taller"><button data-action="show-active-orders" class="priority-card"><span class="priority-icon">${icon('wrench',20)}</span><span class="priority-body"><strong>${open.length} trabajos en curso</strong><small>Revisar el estado de las órdenes</small></span>${icon('chevron',18)}</button><button data-action="show-unpaid-orders" class="priority-card"><span class="priority-icon priority-amber">${icon('wallet',20)}</span><span class="priority-body"><strong>${db.orders.filter(o=>pendingAmount(o)>0).length} cobros pendientes</strong><small>${money(pending)} de saldo por cobrar</small></span>${icon('chevron',18)}</button><button data-action="show-due-vehicles" class="priority-card"><span class="priority-icon priority-blue">${icon('calendar',20)}</span><span class="priority-body"><strong>${db.vehicles.filter(v=>v.nextServiceDate&&daysUntil(v.nextServiceDate)<=30).length} servicios próximos o vencidos</strong><small>Revisar fechas de mantenimiento</small></span>${icon('chevron',18)}</button></div>
  <div class="stats-grid">${stat('Clientes',db.clients.length,'Registrados en el taller','users')}${stat('Trabajos activos',open.length,'En recepción o proceso','wrench')}${stat('Acordado este mes',money(sum(monthly,o=>o.charged)),'Trabajos ingresados este mes','trending')}${stat('Pendiente de cobro',money(pending),'Saldo de todos los trabajos','wallet')}</div>
  ${db.clients.length===0?`<section class="welcome-guide" aria-label="Primeros pasos"><div class="guide-head"><span class="guide-kicker">EMPEZÁ ACÁ</span><h2>Tu primer trabajo, en tres pasos</h2><p>Armá el historial del taller desde la primera visita.</p></div><div class="guide-steps"><div><span>01</span><strong>Registrá a tu cliente</strong><small>Guardá su nombre y contacto.</small></div><div><span>02</span><strong>Sumá su vehículo</strong><small>Identificalo por patente y modelo.</small></div><div><span>03</span><strong>Cargá el servicio</strong><small>Anotá diagnóstico, costos y cobros.</small></div></div>${button(`${icon('plus')} Crear primer cliente`,'new-client','btn-primary')}</section>`:''}
  <div class="dashboard-grid"><div class="card"><div class="card-heading"><div><h3>Últimos trabajos</h3><p>Actividad reciente de tu taller</p></div><button class="link-btn" data-nav="trabajos">Ver todos ${icon('arrow',13)}</button></div>${recent.length?`<div class="table-scroll">${renderOrderTable(recent,true)}</div>`:empty('Todavía no hay trabajos','Registrá el primero para construir el historial de cada vehículo.',button('Nuevo trabajo','new-order','btn-primary'))}</div>
  <div class="card"><div class="card-heading"><div><h3>Próximos servicios</h3><p>Vencidos o dentro de los próximos 30 días</p></div>${icon('calendar',19)}</div>${due.length?`<div class="detail-list">${due.map(v=>{const d=daysUntil(v.nextServiceDate);return `<button class="detail-row table-link" data-nav="vehiculo" data-id="${esc(v.id)}"><div class="detail-icon">${icon('car')}</div><div class="detail-content"><strong>${esc(fullVehicle(v))}</strong><small>${esc(v.plate)} · ${formatDate(v.nextServiceDate)}</small></div><span class="pill pill-${d<0?'overdue':d<=7?'soon':'normal'}">${d<0?'Vencido':d===0?'Hoy':`En ${d} días`}</span></button>`;}).join('')}</div>`:empty('Todo al día','No hay servicios programados para los próximos 30 días.','','calendar')}</div></div>
  <div class="dashboard-grid" style="margin-top:21px"><div class="card card-pad"><div class="card-heading" style="padding:0 0 15px"><div><h3>Movimiento de los últimos 6 meses</h3><p>Importes cargados y costos estimados</p></div>${icon('chart',19)}</div>${renderChart()}</div><div class="card card-pad"><h3 style="font-size:15px;margin-bottom:15px">Accesos rápidos</h3><div style="display:grid;gap:10px">${button(`${icon('car')} Agregar vehículo`,'new-vehicle','btn-secondary')}${button(`${icon('users')} Agregar cliente`,'new-client','btn-secondary')}${button(`${icon('download')} Descargar respaldo`,'export-json','btn-secondary')}</div><p class="muted-sm" style="line-height:1.65;margin-top:17px;margin-bottom:0">${desktopInvoke?'Los datos se guardan en SQLite en este equipo. Configurá copias fuera de la PC para protegerlos ante una falla total.':'Los datos viven solo en este navegador. Descargá un respaldo después de cargar trabajos importantes.'}</p></div></div>`;
}
function stat(label,value,foot,symbol){return `<article class="stat-card"><div class="stat-label">${esc(label)}<span class="stat-icon">${icon(symbol,17)}</span></div><div class="stat-value">${esc(value)}</div><div class="stat-foot">${esc(foot)}</div></article>`;}
function renderChart(){
  const months=Array.from({length:6},(_,i)=>{const d=new Date();d.setDate(1);d.setMonth(d.getMonth()-(5-i));const key=isoDate(d).slice(0,7);const orders=db.orders.filter(o=>o.openedAt.startsWith(key));return {label:d.toLocaleDateString('es-AR',{month:'short'}).replace('.',''),revenue:sum(orders,x=>x.charged),cost:sum(orders,x=>amount(x.partsCost)+amount(x.laborCost))};});
  const max=Math.max(1,...months.flatMap(m=>[m.revenue,m.cost]));
  return `<div class="bar-chart" role="img" aria-label="Gráfico comparativo de importes acordados y costos de los últimos seis meses">${months.map(m=>`<div class="bar-group"><div class="bar-wrap"><span class="bar" style="--h:${Math.max(2,m.revenue/max*100)}%" title="Acordado: ${esc(money(m.revenue))}"></span><span class="bar cost" style="--h:${Math.max(2,m.cost/max*100)}%" title="Costos: ${esc(money(m.cost))}"></span></div><label>${esc(m.label)}</label></div>`).join('')}</div><div class="chart-legend"><span><i class="legend-dot"></i> Acordado</span><span><i class="legend-dot dark"></i> Costos</span></div>`;
}
function renderOrderTable(orders,compact=false){
  const desktop=`<div class="order-table-wrap"><table class="data-table"><thead><tr><th>Trabajo / vehículo</th><th>Fecha</th><th>Estado</th>${compact?'':`<th>Cobrado</th><th>Pendiente</th>`}<th style="width:40px"></th></tr></thead><tbody>${orders.map(o=>{const v=findVehicle(o.vehicleId);return `<tr><td><button class="table-link" data-nav="trabajo" data-id="${esc(o.id)}"><span class="main-cell">${esc(o.title)}</span><span class="sub-cell">${esc(fullVehicle(v))} · ${esc(v?.plate||'')}</span></button></td><td>${formatDate(o.openedAt)}</td><td>${pill(o.status)}</td>${compact?'':`<td><strong>${money(o.paid)}</strong></td><td><span class="${pendingAmount(o)>0?'text-negative':''}">${money(pendingAmount(o))}</span></td>`}<td><button class="btn btn-ghost icon-btn" aria-label="Abrir trabajo" data-nav="trabajo" data-id="${esc(o.id)}">${icon('chevron')}</button></td></tr>`;}).join('')}</tbody></table></div>`;
  const mobile=`<div class="order-card-list">${orders.map(o=>{const v=findVehicle(o.vehicleId);return `<button class="order-mobile-card" data-nav="trabajo" data-id="${esc(o.id)}"><span class="order-mobile-top"><span class="order-mobile-title">${esc(o.title)}</span>${pill(o.status)}</span><span class="order-mobile-vehicle">${icon('car',14)} ${esc(v?.plate||'')} · ${esc(fullVehicle(v))}</span><span class="order-mobile-bottom"><span>${icon('calendar',14)} ${formatDate(o.openedAt)}</span>${compact?'':`<span class="${pendingAmount(o)>0?'text-negative':'text-positive'}">${pendingAmount(o)>0?'Saldo '+money(pendingAmount(o)):'Sin saldo'}</span>`}${icon('chevron',16)}</span></button>`;}).join('')}</div>`;
  return `<div class="responsive-orders">${desktop}${mobile}</div>`;
}
// El buscador de cada listado permanece montado al actualizar resultados: no pierde foco ni cursor.
function listToolbar(section, placeholder, filters = []) {
  const value = listQueries[section];
  const current = section === 'clientes' ? clientFilter : section === 'vehiculos' ? vehicleFilter : activeFilter;
  return `<section class="list-tools" aria-label="Buscar y filtrar ${section}"><div class="list-tools-row"><div class="list-search-field">${icon('search',18)}<label class="sr-only" for="list-search">Buscar en ${section}</label><input id="list-search" data-list-search="${section}" type="search" placeholder="${esc(placeholder)}" value="${esc(value)}" autocomplete="off" enterkeyhint="search"/><button type="button" class="list-clear" data-action="clear-list-search" aria-label="Limpiar búsqueda" ${value?'':'hidden'}>${icon('x',17)}</button></div></div>${filters.length?`<div class="filters list-filters" role="group" aria-label="Filtrar ${section}">${filters.map(([key,label])=>`<button class="filter-tab ${current===key?'active':''}" type="button" data-list-filter="${key}" aria-pressed="${current===key}">${label}</button>`).join('')}</div>`:''}</section>`;
}
function listFooter(result,section){
  const label = result.total === 1 ? 'resultado' : 'resultados';
  return `<div class="list-count" role="status">${result.total ? `Mostrando ${result.first}–${result.last} de ${result.total} ${label}` : `0 ${label}`}</div>${result.pages>1?`<nav class="list-pagination" aria-label="Páginas de ${section}"><button class="btn btn-secondary btn-sm" type="button" data-list-page="${result.page-1}" ${result.page<=1?'disabled':''}>${icon('back',15)} Anterior</button><span>Página ${result.page} de ${result.pages}</span><button class="btn btn-secondary btn-sm" type="button" data-list-page="${result.page+1}" ${result.page>=result.pages?'disabled':''}>Siguiente ${icon('arrow',15)}</button></nav>`:''}`;
}
function listNoMatch(section){
  return empty('No encontramos coincidencias', 'Probá con otra patente, nombre o palabra, o borrá los filtros.',button('Limpiar búsqueda y filtros','clear-list-filters','btn-secondary'),'search');
}
function renderListResults(section) {
  if(section==='clientes'){
    let items=[...db.clients].sort((a,b)=>a.name.localeCompare(b.name,'es'));
    items=items.filter(c=>matchesClient(c,listQueries.clientes) && (clientFilter==='all'||(clientFilter==='with' ? db.vehicles.some(v=>v.clientId===c.id) : !db.vehicles.some(v=>v.clientId===c.id))));
    const page=paginate(items,listPages.clientes,12);listPages.clientes=page.page;
    return `${listFooter(page,section)}${page.total?`<div class="list-grid">${page.items.map(c=>{const vehicles=db.vehicles.filter(v=>v.clientId===c.id);return `<article class="card person-card"><div class="card-top"><span class="avatar">${esc(initials(c.name))}</span><div class="person-info"><h3>${esc(c.name)}</h3><p>${esc(c.email||c.phone||'Sin datos de contacto')}</p></div><button class="btn btn-ghost icon-btn" data-nav="cliente" data-id="${esc(c.id)}" aria-label="Ver cliente">${icon('arrow',17)}</button></div><div class="card-meta"><span>${icon('phone')} ${esc(c.phone||'Sin teléfono')}</span><span>${icon('car')} ${vehicles.length} vehículo${vehicles.length===1?'':'s'}</span></div><div class="card-bottom"><small>Desde ${formatDate(c.createdAt)}</small>${button('Ver ficha','view-client','btn-secondary btn-sm',`data-id="${esc(c.id)}"`)}</div></article>`;}).join('')}</div>`:db.clients.length?listNoMatch(section):empty('Tu lista de clientes está vacía','Agregá un cliente y luego sus vehículos para empezar a registrar servicios.',button('Agregar cliente','new-client','btn-primary'),'users')}`;
  }
  if(section==='vehiculos'){
    let items=[...db.vehicles].sort((a,b)=>fullVehicle(a).localeCompare(fullVehicle(b),'es'));
    items=items.filter(v=>matchesVehicle(v,vehicleClient(v),listQueries.vehiculos) && (vehicleFilter==='all'||(vehicleFilter==='due'&&v.nextServiceDate&&daysUntil(v.nextServiceDate)<=30)||(vehicleFilter==='overdue'&&v.nextServiceDate&&daysUntil(v.nextServiceDate)<0)||(vehicleFilter==='soon'&&v.nextServiceDate&&daysUntil(v.nextServiceDate)>=0&&daysUntil(v.nextServiceDate)<=30)||(vehicleFilter==='undated'&&!v.nextServiceDate)));
    const page=paginate(items,listPages.vehiculos,12);listPages.vehiculos=page.page;
    return `${listFooter(page,section)}${page.total?`<div class="list-grid">${page.items.map(v=>{const c=vehicleClient(v);const n=db.orders.filter(o=>o.vehicleId===v.id).length;const due=v.nextServiceDate?daysUntil(v.nextServiceDate):null;return `<article class="card vehicle-card"><div class="card-top"><span class="avatar blue">${icon('car',23)}</span><div class="person-info"><h3>${esc(fullVehicle(v))} <span class="muted">${esc(v.year||'')}</span></h3><p>${esc(c?.name||'Cliente no disponible')}</p></div><span class="plate">${esc(v.plate)}</span></div><div class="card-meta"><span>${icon('gauge')} ${num(v.mileage)} km</span><span>${icon('clipboard')} ${n} trabajo${n===1?'':'s'}</span></div><div class="card-bottom"><small>${due!==null?(due<0?'Servicio vencido':`Próx. servicio: ${formatDate(v.nextServiceDate)}`):'Sin servicio programado'}</small>${button('Ver historial','view-vehicle','btn-secondary btn-sm',`data-id="${esc(v.id)}"`)}</div></article>`;}).join('')}</div>`:db.vehicles.length?listNoMatch(section):empty('Sin vehículos todavía','Primero registrá un cliente y después vinculá su automóvil.',button('Agregar vehículo','new-vehicle','btn-primary'),'car')}`;
  }
  let orders=[...db.orders].sort((a,b)=>b.openedAt.localeCompare(a.openedAt));
  orders=orders.filter(o=>{
    const v=findVehicle(o.vehicleId);
    return matchesOrder(o,v,vehicleClient(v),listQueries.trabajos) && (activeFilter==='all'||(activeFilter==='active'&&['received','progress','waiting'].includes(o.status))||(activeFilter==='closed'&&['done','delivered'].includes(o.status))||(activeFilter==='unpaid'&&pendingAmount(o)>0));
  });
  const page=paginate(orders,listPages.trabajos,20);listPages.trabajos=page.page;
  return `${listFooter(page,section)}<div class="card">${page.total?`<div class="table-scroll">${renderOrderTable(page.items)}</div>`:db.orders.length?listNoMatch(section):empty('No hay trabajos para mostrar','Creá tu primer trabajo y vas a verlo acá.',button('Nuevo trabajo','new-order','btn-primary'),'clipboard')}</div>`;
}
function refreshListResults(){
  const section=route().page;
  if(!Object.hasOwn(listQueries,section))return;
  const target=document.querySelector('#list-results');if(!target)return;
  target.innerHTML=renderListResults(section);
  const clear=document.querySelector('.list-clear');if(clear)clear.hidden=!listQueries[section];
  const active=section==='clientes'?clientFilter:section==='vehiculos'?vehicleFilter:activeFilter;
  document.querySelectorAll('[data-list-filter]').forEach(b=>{const selected=b.dataset.listFilter===active;b.classList.toggle('active',selected);b.setAttribute('aria-pressed',String(selected));});
}
function renderClients(){
  return `${demoBanner()}${sectionHead('Clientes',`${db.clients.length} clientes registrados. Cada cliente puede tener varios vehículos.`,button(`${icon('plus')} Agregar cliente`,'new-client','btn-primary'))}${listToolbar('clientes','Nombre, teléfono o email…',[['all','Todos'],['with','Con vehículos'],['without','Sin vehículos']])}<div id="list-results">${renderListResults('clientes')}</div>`;
}
function renderVehicles(){
  return `${demoBanner()}${sectionHead('Vehículos',`${db.vehicles.length} vehículos asociados a tus clientes.`,button(`${icon('plus')} Agregar vehículo`,'new-vehicle','btn-primary'))}${listToolbar('vehiculos','Patente, marca, modelo o cliente…',[['all','Todos'],['due','Vencidos y próximos'],['overdue','Vencidos'],['soon','Próximos 30 días'],['undated','Sin fecha']])}<div id="list-results">${renderListResults('vehiculos')}</div>`;
}
function renderOrders(){
  return `${demoBanner()}${sectionHead('Trabajos y servicios',`${db.orders.length} registros en el historial. Registrá desde un diagnóstico hasta una reparación completa.`,button(`${icon('plus')} Nuevo trabajo`,'new-order','btn-primary'))}${listToolbar('trabajos','Trabajo, patente o cliente…',[['all','Todos'],['active','Activos'],['closed','Terminados'],['unpaid','Con saldo']])}<div id="list-results">${renderListResults('trabajos')}</div>`;
}
function renderFinance(){
  const charged=sum(db.orders,o=>o.charged),paid=sum(db.orders,o=>o.paid),cost=sum(db.orders,o=>amount(o.partsCost)+amount(o.laborCost)),pending=sum(db.orders,pendingAmount);
  return `${demoBanner()}${sectionHead('Resumen económico','Un panorama simple de ingresos, costos y saldos. No reemplaza un sistema contable.',button(`${icon('download')} Exportar CSV`,'export-csv','btn-secondary'))}
  <div class="stats-grid">${stat('Importes acordados',money(charged),'Total cargado en los trabajos','wallet')}${stat('Pagos recibidos',money(paid),'Monto efectivamente abonado','trending')}${stat('Costos registrados',money(cost),'Repuestos y mano de obra','wrench')}${stat('Saldo pendiente',money(pending),'Pendiente de cobrar','clock')}</div>
  <div class="dashboard-grid"><div class="card card-pad"><h3 style="font-size:15px">Evolución mensual</h3><p class="section-intro">Importes acordados frente a costos registrados</p>${renderChart()}</div><div class="card card-pad"><h3 style="font-size:15px">Margen estimado</h3><div class="money-box highlight" style="margin:17px 0"><small>Importes cargados − costos</small><strong class="${charged-cost<0?'text-negative':'text-positive'}">${money(charged-cost)}</strong></div><p class="muted-sm" style="line-height:1.8">Es una aproximación, no una utilidad neta: no contempla alquiler, impuestos, herramientas, gastos generales ni el criterio de reconocimiento contable. La mano de obra aquí se considera un costo interno.</p></div></div>
  <div class="card finance-records"><div class="card-heading"><div><h3>Detalle por trabajo</h3><p>Consultá importes, costos y saldos por intervención</p></div></div>
  <div class="finance-toolbar"><div class="list-search-field">${icon('search',18)}<label class="sr-only" for="finance-search">Buscar en movimientos</label><input id="finance-search" type="search" placeholder="Buscar trabajo, patente o cliente…" value="${esc(financeQuery)}" autocomplete="off" /></div><div class="finance-tabs" role="group" aria-label="Filtrar movimientos">${[['all','Todos'],['unpaid','Con saldo'],['negative','Margen negativo']].map(([v,t])=>`<button type="button" data-finance-filter="${v}" aria-pressed="${financeFilter===v}" class="filter-tab ${financeFilter===v?'active':''}">${t}</button>`).join('')}</div></div><div id="finance-results">${renderFinanceResults()}</div></div>`;
}
function renderFinanceResults(){
  let orders=[...db.orders].sort((a,b)=>b.openedAt.localeCompare(a.openedAt));
  orders=orders.filter(o=>{const v=findVehicle(o.vehicleId);return matchesOrder(o,v,vehicleClient(v),financeQuery)&&(financeFilter==='all'||financeFilter==='unpaid'&&pendingAmount(o)>0||financeFilter==='negative'&&margin(o)<0);});
  const page=paginate(orders,financePage,15);financePage=page.page;
  return `${listFooter(page,'finanzas')}${page.total?`<div class="table-scroll finance-table-wrap"><table class="data-table"><thead><tr><th>Trabajo</th><th>Costo</th><th>Acordado</th><th>Pagado</th><th>Saldo</th><th>Margen estimado</th></tr></thead><tbody>${page.items.map(o=>`<tr><td><button class="table-link" data-nav="trabajo" data-id="${esc(o.id)}"><span class="main-cell">${esc(o.title)}</span><span class="sub-cell">${esc(findVehicle(o.vehicleId)?.plate||'')} · ${formatDate(o.openedAt)}</span></button></td><td>${money(amount(o.partsCost)+amount(o.laborCost))}</td><td>${money(o.charged)}</td><td>${money(o.paid)}</td><td>${money(pendingAmount(o))}</td><td class="${margin(o)<0?'text-negative':'text-positive'}">${money(margin(o))}</td></tr>`).join('')}</tbody></table></div><div class="finance-mobile-list">${page.items.map(o=>{const v=findVehicle(o.vehicleId);return `<button class="finance-mobile-card" data-nav="trabajo" data-id="${esc(o.id)}"><span class="order-mobile-top"><strong>${esc(o.title)}</strong>${icon('chevron',17)}</span><span class="order-mobile-vehicle">${esc(v?.plate||'')} · ${formatDate(o.openedAt)}</span><span class="finance-mobile-metrics"><span><small>Acordado</small><strong>${money(o.charged)}</strong></span><span><small>Pagado</small><strong>${money(o.paid)}</strong></span><span><small>Saldo</small><strong class="${pendingAmount(o)>0?'text-negative':'text-positive'}">${money(pendingAmount(o))}</strong></span></span></button>`;}).join('')}</div>`:db.orders.length?listNoMatch('finanzas'):empty('Sin movimientos todavía','Los importes que cargues en tus trabajos aparecerán acá.','','wallet')}`;
}
function refreshFinanceResults(){const node=document.querySelector('#finance-results');if(node)node.innerHTML=renderFinanceResults();document.querySelectorAll('[data-finance-filter]').forEach(el=>{const active=el.dataset.financeFilter===financeFilter;el.classList.toggle('active',active);el.setAttribute('aria-pressed',String(active));});}

function renderClientDetail(id){
  const c=findClient(id); if(!c)return notFound('cliente','clientes');
  const vehicles=db.vehicles.filter(v=>v.clientId===id);
  const orders=db.orders.filter(o=>vehicles.some(v=>v.id===o.vehicleId)).sort((a,b)=>b.openedAt.localeCompare(a.openedAt));
  return `${back('clientes','Clientes')}${sectionHead(c.name,`${vehicles.length} vehículo${vehicles.length===1?'':'s'} · ${orders.length} trabajo${orders.length===1?'':'s'}`,`${button(`${icon('pencil')} Editar`,'edit-client','btn-secondary',`data-id="${esc(id)}"`)} ${desktopInvoke?button(`${icon('download')} Excel para cliente`,'export-client-excel','btn-secondary',`data-id="${esc(id)}"`):''} ${button(`${icon('plus')} Agregar vehículo`,'new-vehicle','btn-primary',`data-client="${esc(id)}"`)}`)}<div class="detail-layout"><div><div class="card card-pad detail-section"><h3>Información de contacto</h3><div class="info-grid">${descView('Teléfono',phoneLink(c.phone))}${descView('Correo',c.email?`<a href="mailto:${encodeURIComponent(c.email)}">${esc(c.email)}</a>`:'—')}${descView('Registrado',formatDate(c.createdAt))}${descView('Vehículos',String(vehicles.length))}</div>${c.notes?`<div class="divider"></div><p class="section-intro">${esc(c.notes)}</p>`:''}</div><div class="card detail-section"><div class="card-heading"><h3>Vehículos asociados</h3></div>${vehicles.length?`<div class="detail-list">${vehicles.map(v=>`<button class="detail-row table-link" data-nav="vehiculo" data-id="${esc(v.id)}"><span class="detail-icon">${icon('car')}</span><div class="detail-content"><strong>${esc(fullVehicle(v))}</strong><small>${esc(v.plate)} · ${num(v.mileage)} km</small></div>${icon('chevron',17)}</button>`).join('')}</div>`:empty('Sin vehículos','Agregá el primer automóvil del cliente.')}</div></div><div><div class="card detail-section"><div class="card-heading"><h3>Últimos trabajos</h3></div>${orders.length?`<div class="detail-list">${orders.slice(0,8).map(o=>`<button class="detail-row table-link" data-nav="trabajo" data-id="${esc(o.id)}"><div class="detail-content"><strong>${esc(o.title)}</strong><small>${formatDate(o.openedAt)} · ${esc(findVehicle(o.vehicleId)?.plate||'')}</small></div>${icon('chevron',16)}</button>`).join('')}</div>`:empty('Sin trabajos','El historial aparecerá al cargar el primer servicio.')}</div><div class="card card-pad"><h3>Administración</h3><p class="muted-sm">Solo podés eliminar un cliente cuando no tenga vehículos asociados.</p>${button(`${icon('trash')} Eliminar cliente`,'delete-client','btn-danger-ghost btn-sm',`data-id="${esc(id)}"`)}</div></div></div>`;
}
function renderHistoryToolbar(){
  return `<div class="history-toolbar" aria-label="Buscar en el historial del vehículo"><div class="history-search">${icon('search',17)}<label class="sr-only" for="history-search">Buscar intervenciones de este vehículo</label><input id="history-search" type="search" autocomplete="off" placeholder="Buscar servicio, diagnóstico, fecha…" value="${esc(historyQuery)}" /></div><div class="history-filters" role="group" aria-label="Filtrar servicios">${[['all','Todos'],['active','En curso'],['closed','Finalizados']].map(([key,label])=>`<button type="button" data-history-filter="${key}" class="filter-tab ${historyStatus===key?'active':''}" aria-pressed="${historyStatus===key}">${label}</button>`).join('')}</div></div>`;
}
function renderVehicleHistory(id){
  const works=db.orders.filter(o=>o.vehicleId===id);
  const matches=filterVehicleHistory(works,historyQuery,historyStatus);
  const page=paginate(matches,historyPage,8);historyPage=page.page;
  return `<div class="history-count" role="status">${page.total} de ${works.length} intervenciones</div>
  ${page.total?`<div class="timeline history-timeline">${page.items.map(o=>`<article class="timeline-item"><div class="history-item-top"><button class="table-link" data-nav="trabajo" data-id="${esc(o.id)}"><strong class="timeline-title">${esc(o.title)}</strong></button>${pill(o.status)}</div><div class="timeline-meta">${formatDate(o.openedAt)} · ${esc(o.category)} · ${num(o.mileage)} km</div>${o.description?`<p class="timeline-body history-description">${esc(o.description)}</p>`:''}<button class="history-detail-link" data-nav="trabajo" data-id="${esc(o.id)}">Ver ficha completa ${icon('arrow',14)}</button></article>`).join('')}</div>`:empty('Sin resultados en el historial','Probá otro término o cambiá el filtro.',button('Mostrar todos','clear-history','btn-secondary'),'search')}
  ${page.pages>1?`<div class="history-pagination"><button type="button" class="btn btn-secondary btn-sm" data-history-page="${page.page-1}" ${page.page===1?'disabled':''}>${icon('back',15)} Anterior</button><span>Página ${page.page} de ${page.pages}</span><button type="button" class="btn btn-secondary btn-sm" data-history-page="${page.page+1}" ${page.page===page.pages?'disabled':''}>Siguiente ${icon('arrow',15)}</button></div>`:''}`;
}
function refreshVehicleHistory(){
  const node=document.querySelector('#vehicle-history-results');if(node)node.innerHTML=renderVehicleHistory(route().id);
  document.querySelectorAll('[data-history-filter]').forEach(b=>{const active=b.dataset.historyFilter===historyStatus;b.classList.toggle('active',active);b.setAttribute('aria-pressed',String(active));});
}
function renderVehicleDetail(id){
  const v=findVehicle(id);if(!v)return notFound('vehículo','vehiculos');
  if(historyVehicle!==id){historyVehicle=id;historyQuery='';historyStatus='all';historyPage=1;}
  const c=vehicleClient(v);
  const works=db.orders.filter(o=>o.vehicleId===id).sort((a,b)=>b.openedAt.localeCompare(a.openedAt));
  const latest=works[0],open=works.filter(o=>['received','progress','waiting'].includes(o.status)).length;
  return `${back('vehiculos','Vehículos')}${sectionHead(fullVehicle(v),`${v.plate} · ${v.year||'Año sin especificar'}`,`${button(`${icon('pencil')} Editar vehículo`,'edit-vehicle','btn-secondary',`data-id="${esc(id)}"`)} ${button(`${icon('plus')} Nuevo trabajo`,'new-order','btn-primary',`data-vehicle="${esc(id)}"`)}`)}
  <div class="vehicle-quick-summary" aria-label="Resumen del vehículo"><div><small>Última visita</small><strong>${latest?formatDate(latest.openedAt):'Sin intervenciones'}</strong></div><div><small>Trabajos registrados</small><strong>${works.length}</strong></div><div><small>En curso</small><strong>${open}</strong></div><div><small>Kilometraje actual</small><strong>${num(v.mileage)} km</strong></div></div>
  <div class="detail-layout"><div><div class="card card-pad detail-section"><h3>Ficha del vehículo</h3><div class="info-grid">${descView('Patente',`<span class="plate">${esc(v.plate)}</span>`)}${descView('Cliente',`<button class="table-link" data-nav="cliente" data-id="${esc(v.clientId)}"><span class="main-cell">${esc(c?.name||'—')}</span></button>`)}${descView('Año',esc(v.year||'—'))}${descView('Kilometraje',`${num(v.mileage)} km`)}${descView('Número de chasis (VIN)',esc(v.vin||'—'))}${descView('Color',esc(v.color||'—'))}${descView('Próximo servicio',formatDate(v.nextServiceDate))}${descView('Intervenciones',String(works.length))}</div>${v.notes?`<div class="divider"></div><p class="section-intro">${esc(v.notes)}</p>`:''}</div>
  <section class="card card-pad detail-section"><div class="card-heading history-heading"><div><h3>Historial de servicios</h3><p>Todo lo que se hizo en este vehículo, organizado por fecha.</p></div><span class="tag">${works.length} registros</span></div>${works.length?`${renderHistoryToolbar()}<div id="vehicle-history-results">${renderVehicleHistory(id)}</div>`:empty('Sin historial','Agregá el primer trabajo para construir el historial del vehículo.',button('Registrar primer trabajo','new-order','btn-primary',`data-vehicle="${esc(id)}"`))}</section></div>
  <div><div class="card card-pad detail-section"><h3>Propietario</h3><div class="card-top"><div class="avatar">${esc(initials(c?.name))}</div><div class="person-info"><h3>${esc(c?.name||'Sin propietario')}</h3><p>${esc(c?.phone||'Sin teléfono')}</p></div></div><div class="vehicle-owner-actions">${button('Abrir ficha del cliente','view-client','btn-secondary',`data-id="${esc(c?.id||'')}"`)}${c?.phone?phoneLink(c.phone):''}</div></div><div class="card card-pad"><h3>Administración</h3><p class="muted-sm">Solo podés eliminar un vehículo sin trabajos asociados.</p>${button(`${icon('trash')} Eliminar vehículo`,'delete-vehicle','btn-danger-ghost btn-sm',`data-id="${esc(id)}"`)}</div></div></div>`;
}
function renderOrderDetail(id){
  const o=findOrder(id);if(!o)return notFound('trabajo','trabajos');const v=findVehicle(o.vehicleId);const c=vehicleClient(v);
  const cost=amount(o.partsCost)+amount(o.laborCost);
  return `${back('trabajos','Trabajos')}${sectionHead(o.title,`${fullVehicle(v)} · ${v?.plate||''} · ${formatDate(o.openedAt)}`,`${button(`${icon('pencil')} Editar trabajo`,'edit-order','btn-secondary',`data-id="${esc(id)}"`)} ${button(`${icon('printer')} Resumen para cliente`,'print-order','btn-secondary',`data-id="${esc(id)}"`)}`)}<div class="detail-layout"><div><div class="card card-pad detail-section"><div style="display:flex;align-items:center;justify-content:space-between;gap:10px"><h3 style="margin:0">Detalle de la intervención</h3>${pill(o.status)}</div><div class="divider"></div><div class="info-grid">${descView('Categoría',esc(o.category))}${descView('Fecha de ingreso',formatDate(o.openedAt))}${descView('Kilometraje registrado',`${num(o.mileage)} km`)}${descView('Fecha de cierre',formatDate(o.completedAt))}</div><div class="divider"></div><h3 style="font-size:13px">Diagnóstico / Descripción</h3><p class="section-intro" style="white-space:pre-wrap">${esc(o.description||'Sin descripción adicional.')}</p>${o.notes?`<h3 style="font-size:13px">Notas</h3><p class="section-intro" style="white-space:pre-wrap">${esc(o.notes)}</p>`:''}</div><div class="card card-pad detail-section"><h3>Seguimiento</h3><p class="section-intro">Actualizá el estado del trabajo sin tener que abrir el formulario completo.</p><div class="field"><label for="status-quick">Estado actual</label><select id="status-quick" data-status-order="${esc(id)}">${Object.entries(STATUSES).map(([k,label])=>`<option value="${k}" ${k===o.status?'selected':''}>${label}</option>`).join('')}</select></div></div></div><div><div class="card card-pad detail-section"><h3>Balance del trabajo</h3><div class="money-grid" style="margin-top:18px"><div class="money-box"><small>Costo de repuestos</small><strong>${money(o.partsCost)}</strong></div><div class="money-box"><small>Costo de mano de obra</small><strong>${money(o.laborCost)}</strong></div><div class="money-box"><small>Costo total</small><strong>${money(cost)}</strong></div><div class="money-box"><small>Importe acordado</small><strong>${money(o.charged)}</strong></div><div class="money-box highlight"><small>Pagado</small><strong>${money(o.paid)}</strong></div><div class="money-box ${pendingAmount(o)?'due':''}"><small>Saldo pendiente</small><strong>${money(pendingAmount(o))}</strong></div></div><div class="divider"></div><div class="info-item"><small>Margen bruto estimado</small><strong class="${margin(o)<0?'text-negative':'text-positive'}" style="font-size:22px">${money(margin(o))}</strong></div><div style="margin-top:20px">${button(`${icon('plus')} Registrar cobro`,'add-payment',pendingAmount(o)>0?'btn-primary':'btn-secondary',`data-id="${esc(id)}" ${pendingAmount(o)===0?'disabled':''}`)}</div></div><div class="card card-pad detail-section"><h3>Vehículo y cliente</h3><div class="info-grid">${descView('Vehículo',esc(fullVehicle(v)))}${descView('Patente',esc(v?.plate||''))}${descView('Cliente',esc(c?.name||''))}${descView('Contacto',phoneLink(c?.phone))}</div><div style="margin-top:20px">${button('Ver vehículo','view-vehicle','btn-secondary btn-sm',`data-id="${esc(v?.id||'')}"`)}</div></div><div class="card card-pad">${button(`${icon('trash')} Eliminar trabajo`,'delete-order','btn-danger-ghost btn-sm',`data-id="${esc(id)}"`)}</div></div></div>`;
}
function notFound(item,to){return `${back(to,'Volver')}${empty('No encontramos este '+item,'Es posible que el registro se haya eliminado.')}`;}
function back(to,label){return `<button class="back-btn" data-nav="${to}">${icon('back',15)} Volver a ${esc(label)}</button>`;}
function renderSearch(){
  const cs=db.clients.filter(c=>matchesClient(c,activeQuery));
  const vs=db.vehicles.filter(v=>matchesVehicle(v,vehicleClient(v),activeQuery));
  const os=db.orders.filter(o=>{const v=findVehicle(o.vehicleId);return matchesOrder(o,v,vehicleClient(v),activeQuery);});
  return `${sectionHead('Resultados de búsqueda',`Buscando “${esc(activeQuery)}” en clientes, vehículos y trabajos.`)}<div class="card"><div class="card-heading"><h3>Clientes <span class="tag">${cs.length}</span></h3></div>${cs.length?`<div class="detail-list">${cs.slice(0,8).map(c=>`<button class="detail-row table-link" data-nav="cliente" data-id="${esc(c.id)}"><div class="detail-icon">${icon('users')}</div><div class="detail-content"><strong>${esc(c.name)}</strong><small>${esc(c.phone||c.email||'Sin contacto')}</small></div>${icon('chevron')}</button>`).join('')}</div>`:empty('Ningún cliente coincide','Probá con otro término.','','users')}</div><div class="card" style="margin-top:18px"><div class="card-heading"><h3>Vehículos <span class="tag">${vs.length}</span></h3></div>${vs.length?`<div class="detail-list">${vs.slice(0,8).map(v=>`<button class="detail-row table-link" data-nav="vehiculo" data-id="${esc(v.id)}"><div class="detail-icon">${icon('car')}</div><div class="detail-content"><strong>${esc(fullVehicle(v))}</strong><small>${esc(v.plate)} · ${esc(findClient(v.clientId)?.name||'')}</small></div>${icon('chevron')}</button>`).join('')}</div>`:empty('Ningún vehículo coincide','Buscá por patente, marca o modelo.','','car')}</div><div class="card" style="margin-top:18px"><div class="card-heading"><h3>Trabajos <span class="tag">${os.length}</span></h3></div>${os.length?`<div class="table-scroll">${renderOrderTable(os.slice(0,8))}</div>`:empty('Ningún trabajo coincide','Buscá por nombre del trabajo, categoría o patente.')}</div>`;
}
function renderSettings(){
  return `${sectionHead('Configuración y respaldos','Personalizá AutoTrace y administrá tu información.')}
  ${memoryOnly?'<p class="notice warning">Este navegador no permite almacenar datos. No cargues información real en esta sesión.</p>':''}
  <div class="settings-grid"><div><div class="card settings-block detail-section"><h3>Preferencias del taller</h3><p>Estos ajustes afectan la visualización, no modifican tus importes.</p><form id="settings-form" class="form-grid"><div class="field span-2"><label for="garageName">Nombre del taller</label><input id="garageName" name="garageName" maxlength="100" value="${esc(db.settings.garageName)}" required /></div><div class="field"><label for="currency">Moneda</label><select id="currency" name="currency">${['ARS','USD','EUR'].map(c=>`<option value="${c}" ${db.settings.currency===c?'selected':''}>${c}</option>`).join('')}</select></div><div class="field" style="justify-content:flex-end"> <button class="btn btn-primary" type="submit">Guardar preferencias</button></div></form></div><div class="card settings-block"><h3>Datos locales</h3><p>Descargá una copia completa de clientes, vehículos, trabajos y preferencias. Importar reemplaza todos los registros actuales.</p><div class="settings-actions">${button(`${icon('download')} Exportar JSON`,'export-json','btn-primary')}${button(`${icon('upload')} Importar JSON`,'import-json','btn-secondary')}${button(`${icon('download')} Exportar CSV`,'export-csv','btn-secondary')}${desktopInvoke?`${button(`${icon('download')} Excel completo`,'export-excel-desktop','btn-primary')}${button(`${icon('upload')} Importar Excel`,'import-excel-desktop','btn-secondary')}${button(`${icon('shield')} Crear respaldo ahora`,'backup-desktop-now','btn-secondary')}`:''}</div><div class="notice" style="margin-top:23px">${icon('shield',17)} ${desktopInvoke?'SQLite local. Las copias automáticas SQLite + Excel se guardan EN ESTA COMPUTADORA. Para protegerte ante pérdida del equipo, exportá una copia a un disco externo. El Excel contiene datos personales y costos internos.':'Los registros se guardan con IndexedDB en este navegador. No se envían a GitHub ni a servidores de AutoTrace. Si borrás datos del sitio o cambiás de dispositivo, necesitarás el respaldo.'}</div></div></div><div><div class="card settings-block detail-section"><h3>Estado de la instalación</h3><p>Esta versión es una PWA instalable y funciona sin conexión después de abrirla por primera vez.</p><div class="info-grid">${descView('Clientes',String(db.clients.length))}${descView('Vehículos',String(db.vehicles.length))}${descView('Trabajos',String(db.orders.length))}${descView('Almacenamiento',desktopInvoke?'SQLite local':memoryOnly?'No disponible':'Local (IndexedDB)')}${desktopInvoke?descView('Último respaldo local',lastDesktopBackup?.lastBackupUnixMs?new Date(Number(lastDesktopBackup.lastBackupUnixMs)).toLocaleString('es-AR'):'Pendiente'):''}</div></div><div class="card settings-block"><h3>Zona de mantenimiento</h3><p>Las acciones siguientes eliminan datos. Antes de continuar, exportá un respaldo JSON.</p><div class="settings-actions">${db.settings.demo?button('Eliminar datos de muestra','reset-demo','btn-secondary'):''}${button(`${icon('trash')} Borrar todos los datos`,'clear-all','btn-danger-ghost')}</div></div><div class="notice warning" style="margin-top:19px">${desktopInvoke?'Las copias automáticas locales NO protegen ante robo, fallo total del disco o incendio. Copiá el Excel completo y la copia SQLite a un medio externo. No hay cifrado automático ni backups en la nube.':'Esta versión está pensada para pruebas y uso individual. No incluye cuentas, cifrado, facturación fiscal, copias automáticas ni sincronización entre dispositivos.'}</div></div></div>`;
}
const input = (label,name,value='',opts={})=>`<div class="field ${opts.full?'span-2':''}"><label for="f-${name}">${esc(label)}${opts.required?' *':''}</label>${opts.textarea?`<textarea id="f-${name}" name="${name}" maxlength="${opts.maxlength||3000}" placeholder="${esc(opts.placeholder||'')}">${esc(value)}</textarea>`:opts.select?`<select id="f-${name}" name="${name}" ${opts.required?'required':''}>${opts.select.map(x=>`<option value="${esc(x.value)}" ${String(x.value)===String(value)?'selected':''}>${esc(x.label)}</option>`).join('')}</select>`:`<input id="f-${name}" type="${opts.type||'text'}" name="${name}" value="${esc(value)}" ${opts.required?'required':''} ${opts.min!==undefined?`min="${opts.min}"`:''} ${opts.max!==undefined?`max="${opts.max}"`:''} ${opts.step!==undefined?`step="${opts.step}"`:''} maxlength="${opts.maxlength||120}" placeholder="${esc(opts.placeholder||'')}" />`}${opts.hint?`<small>${esc(opts.hint)}</small>`:''}</div>`;
function showModal(title,subtitle,kind,fields,id=''){
  const dialog=document.querySelector('#form-dialog');
  document.querySelector('#modal-content').innerHTML=`<form id="entity-form" data-kind="${kind}" data-id="${esc(id)}"><div class="modal-head"><div><h2 id="modal-title">${esc(title)}</h2><p>${esc(subtitle)}</p></div><button class="modal-close" type="button" data-action="close-modal" aria-label="Cerrar formulario">${icon('x')}</button></div><div class="modal-body"><div class="form-grid">${fields}</div></div><div class="modal-actions"><button type="button" class="btn btn-secondary" data-action="close-modal">Cancelar</button><button type="submit" class="btn btn-primary">${icon('check')} Guardar</button></div></form>`;
  dialog.showModal();
  // Al reutilizar el diálogo nunca arrastramos el scroll (horizontal o vertical) del formulario anterior.
  const body=dialog.querySelector('.modal-body');
  body.scrollTop=0;
  body.scrollLeft=0;
  modalOriginal=modalSnapshot(dialog.querySelector('#entity-form'));
  if(matchMedia('(min-width: 851px)').matches) setTimeout(()=>dialog.querySelector('input:not([type=hidden]), select')?.focus(),0);
}
function clientForm(id=''){
  const c=id?findClient(id):null;
  showModal(c?'Editar cliente':'Nuevo cliente','Guardá sus datos de contacto para futuras visitas.','client',[
    input('Nombre y apellido *','name',c?.name,{required:true,full:true,maxlength:120,placeholder:'Ej. Juan Pérez'}),
    input('Teléfono','phone',c?.phone,{type:'tel',placeholder:'Ej. 223 555 0123'}),
    input('Correo electrónico','email',c?.email,{type:'email',placeholder:'nombre@correo.com'}),
    input('Notas del cliente','notes',c?.notes,{textarea:true,full:true,placeholder:'Preferencias, observaciones…'})
  ].join(''),id);
}
function vehicleForm(id='',presetClient=''){
  const v=id?findVehicle(id):null;
  if(!db.clients.length){toast('Primero agregá un cliente para asociar el vehículo.','error');navigate('clientes');return;}
  showModal(v?'Editar vehículo':'Nuevo vehículo','Vinculá un automóvil con uno de tus clientes.','vehicle',[
    clientPicker(v?.clientId||presetClient),
    input('Patente','plate',v?.plate,{required:true,placeholder:'Ej. AF 123 BC',maxlength:12}),
    input('Marca','make',v?.make,{required:true,placeholder:'Ej. Volkswagen'}),
    input('Modelo','model',v?.model,{required:true,placeholder:'Ej. Gol Trend'}),
    input('Año','year',v?.year||'',{type:'number',min:1900,max:new Date().getFullYear()+2}),
    input('Kilometraje actual','mileage',v?.mileage??'',{type:'number',min:0,step:1,placeholder:'Ej. 62.500'}),
    input('VIN / chasis','vin',v?.vin,{placeholder:'Opcional',maxlength:40}),
    input('Color','color',v?.color,{placeholder:'Opcional'}),
    input('Próximo servicio (fecha)','nextServiceDate',v?.nextServiceDate,{type:'date',full:true}),
    input('Observaciones','notes',v?.notes,{textarea:true,full:true,placeholder:'Datos útiles sobre el vehículo…'})
  ].join(''),id);
}
function vehiclePickerOption(v){
  const c=vehicleClient(v);
  return `<button type="button" class="vehicle-picker-option" data-action="pick-vehicle" data-id="${esc(v.id)}"><span class="plate">${esc(v.plate)}</span><span class="vehicle-picker-name"><strong>${esc(fullVehicle(v))}</strong><small>${esc(c?.name||'Cliente sin datos')} · ${num(v.mileage)} km</small></span>${icon('chevron',16)}</button>`;
}
function vehiclePickerResults(query=''){
  const {items,total,searching}=suggestVehicles(db.vehicles,db.clients,db.orders,query);
  const caption=searching?`${total} coincidencia${total===1?'':'s'} · máx. 5 resultados`:`Vehículos recientes · ${Math.min(total,3)} de ${total}`;
  return `<div class="picker-count" role="status">${caption}</div>${items.length?items.map(vehiclePickerOption).join(''):`<div class="picker-empty">Sin coincidencias. Probá con otra patente, marca o cliente.</div>`}${total>items.length?`<p class="picker-more">${searching?'Afiná la búsqueda para encontrar el vehículo.':'Escribí arriba para buscar en todos los vehículos.'}</p>`:''}`;
}
function clientPickerOption(c){
  const n=db.vehicles.filter(v=>v.clientId===c.id).length;
  return `<button type="button" class="client-picker-option" data-action="pick-client" data-id="${esc(c.id)}"><span class="avatar">${esc(initials(c.name))}</span><span><strong>${esc(c.name)}</strong><small>${n} vehículo${n===1?'':'s'} · ${esc(c.phone||'Sin teléfono')}</small></span>${icon('chevron',16)}</button>`;
}
function clientPickerResults(query=''){
  const {items,total,searching}=suggestClients(db.clients,query);
  return `<div class="picker-count" role="status">${searching?`${total} coincidencia${total===1?'':'s'} · máx. 5 resultados`:`Clientes recientes · ${Math.min(total,3)} de ${total}`}</div>${items.length?items.map(clientPickerOption).join(''):`<div class="picker-empty">No encontramos el cliente. Probá con su nombre o teléfono.</div>`}${total>items.length?`<p class="picker-more">${searching?'Escribí más para precisar la búsqueda.':'Buscá por nombre o teléfono para ver los demás.'}</p>`:''}`;
}
function clientPicker(selectedId=''){
  const c=findClient(selectedId);
  return `<div class="client-picker span-2"><label for="client-lookup">Cliente *</label><input type="hidden" id="f-clientId" name="clientId" value="${esc(selectedId)}"><div id="client-selected" class="picker-selected" ${c?'':'hidden'}>${c?`<span class="selected-mark">${icon('check',17)}</span><div><strong>${esc(c.name)}</strong><small>${esc(c.phone||'Sin teléfono')}</small></div><button class="btn btn-secondary btn-sm" type="button" data-action="change-client">Cambiar</button>`:''}</div><div class="client-picker-search" ${c?'hidden':''}><div class="history-search">${icon('search',17)}<input id="client-lookup" type="search" autocomplete="off" placeholder="Nombre o teléfono…" aria-describedby="client-picker-help" aria-controls="client-suggestions"></div><small id="client-picker-help">Seleccioná un cliente para vincular el vehículo.</small><div id="client-suggestions" class="client-picker-list">${clientPickerResults()}</div></div></div>`;
}
function selectClientForVehicle(id){
  const c=findClient(id);const form=document.querySelector('#entity-form');
  if(!c||form?.dataset.kind!=='vehicle')return;
  form.querySelector('#f-clientId').value=id;
  const selected=form.querySelector('#client-selected');
  selected.innerHTML=`<span class="selected-mark">${icon('check',17)}</span><div><strong>${esc(c.name)}</strong><small>${esc(c.phone||'Sin teléfono')}</small></div><button class="btn btn-secondary btn-sm" type="button" data-action="change-client">Cambiar</button>`;
  selected.hidden=false;form.querySelector('.client-picker-search').hidden=true;
  if(matchMedia('(min-width: 851px)').matches)form.querySelector('#f-plate').focus();
}
function vehiclePicker(selectedId=''){
  const selected=findVehicle(selectedId);
  return `<div class="vehicle-picker span-2"><label for="vehicle-lookup">Vehículo *</label><input id="f-vehicleId" name="vehicleId" type="hidden" value="${esc(selectedId)}" /><div id="vehicle-selected" class="vehicle-selected" ${selected?'':'hidden'}>${selected?`<span class="selected-mark">${icon('check',17)}</span><div><strong>${esc(selected.plate)} · ${esc(fullVehicle(selected))}</strong><small>${esc(vehicleClient(selected)?.name||'')} · ${num(selected.mileage)} km</small></div>`:''}<button type="button" class="btn btn-secondary btn-sm" data-action="change-vehicle">Cambiar</button></div><div class="vehicle-picker-search" ${selected?'hidden':''}><div class="history-search">${icon('search',17)}<input id="vehicle-lookup" type="search" placeholder="Patente, marca o cliente…" autocomplete="off" aria-controls="vehicle-suggestions" aria-describedby="vehicle-picker-help" /></div><small id="vehicle-picker-help">Buscá por patente, vehículo o nombre del cliente. Mostramos 3 recientes o hasta 5 coincidencias; buscá para ver más.</small><div id="vehicle-suggestions" class="vehicle-picker-list">${vehiclePickerResults()}</div></div></div>`;
}
function selectVehicleForOrder(id){
  const selected=findVehicle(id);if(!selected)return;
  const form=document.querySelector('#entity-form');if(!form||form.dataset.kind!=='order')return;
  const current=form.querySelector('#f-vehicleId');const prior=current.value;
  current.value=id;
  const block=form.querySelector('#vehicle-selected');block.innerHTML=`<span class="selected-mark">${icon('check',17)}</span><div><strong>${esc(selected.plate)} · ${esc(fullVehicle(selected))}</strong><small>${esc(vehicleClient(selected)?.name||'')} · ${num(selected.mileage)} km</small></div><button type="button" class="btn btn-secondary btn-sm" data-action="change-vehicle">Cambiar</button>`;
  block.hidden=false;form.querySelector('.vehicle-picker-search').hidden=true;
  const mileage=form.elements.namedItem('mileage');const oldVehicle=findVehicle(prior);
  if(!form.dataset.id && (!mileage.value || Number(mileage.value)===Number(oldVehicle?.mileage??0)))mileage.value=selected.mileage;
  form.elements.namedItem('title').focus();
}
function orderForm(id='',presetVehicle=''){
  const o=id?findOrder(id):null;
  if(!db.vehicles.length){toast('Primero agregá un vehículo para registrar un trabajo.','error');navigate('vehiculos');return;}
  const chosen=findVehicle(o?.vehicleId||presetVehicle);
  const hasAdvanced=!!o && !!(o.partsCost||o.laborCost||o.charged||o.paid||o.notes);
  showModal(o?'Editar trabajo':'Nuevo trabajo','Elegí el vehículo, anotá el servicio y listo. Los importes son opcionales.','order',[
    vehiclePicker(o?.vehicleId||presetVehicle),
    input('Trabajo o servicio','title',o?.title,{required:true,full:true,placeholder:'Ej. Revisión de frenos',maxlength:150}),
    input('Categoría','category',o?.category||CATEGORIES[0],{select:CATEGORIES.map(c=>({value:c,label:c})),required:true}),
    input('Estado','status',o?.status||'received',{select:Object.entries(STATUSES).map(([value,label])=>({value,label})),required:true}),
    input('Fecha','openedAt',o?.openedAt||isoDate(new Date()),{type:'date',required:true}),
    input('Kilometraje','mileage',o?.mileage??chosen?.mileage??0,{type:'number',min:0,step:1}),
    input('Diagnóstico y tareas realizadas','description',o?.description,{textarea:true,full:true,placeholder:'Qué sucede, qué revisaste o reparaste…'}),
    `<div class="span-2"><details class="order-extras" ${hasAdvanced?'open':''}><summary>${icon('wallet',18)} Costos, cobros y notas <span class="extras-optional">Opcional</span>${icon('chevron',17)}</summary><div class="form-grid order-extras-grid">${[
      input('Costo de repuestos','partsCost',o?.partsCost??0,{type:'number',min:0,step:'.01'}),
      input('Costo interno de mano de obra','laborCost',o?.laborCost??0,{type:'number',min:0,step:'.01'}),
      input('Importe acordado','charged',o?.charged??0,{type:'number',min:0,step:'.01'}),
      input('Ya pagado','paid',o?.paid??0,{type:'number',min:0,step:'.01',hint:'Total recibido hasta ahora.'}),
      input('Notas adicionales','notes',o?.notes,{textarea:true,full:true,placeholder:'Pendientes o recomendaciones…'})
    ].join('')}</div></details></div>`
  ].join(''),id);
  if(matchMedia('(min-width: 851px)').matches) setTimeout(()=>document.querySelector(chosen?'#f-title':'#vehicle-lookup')?.focus(),0);
}
function paymentForm(id){
  const o=findOrder(id);if(!o)return;
  const pending=pendingAmount(o);if(!pending){toast('Este trabajo no tiene saldo pendiente.');return;}
  showModal('Registrar cobro',`Saldo pendiente: ${money(pending)} · ${o.title}`,'payment',[
    input('Importe recibido','payment',pending,{required:true,type:'number',min:'.01',max:pending,step:'.01',full:true,hint:'Se sumará a los pagos anteriores del trabajo.'})
  ].join(''),id);
}
async function saveForm(form){
  if(!form.reportValidity())return;
  const fd=new FormData(form);const get=k=>String(fd.get(k)||'').trim();
  const kind=form.dataset.kind,id=form.dataset.id,existing=!!id;
  const baseId=id||uid();
  let next,routeTo,message;
  if(kind==='client'){
    next={id:baseId,name:get('name'),phone:get('phone'),email:get('email'),notes:get('notes'),createdAt:findClient(id)?.createdAt||isoDate(new Date())};
    if(!next.name){toast('Ingresá el nombre del cliente.','error');return;}
    routeTo=['cliente',baseId];message=existing?'Cliente actualizado.':'Cliente registrado.';
  }else if(kind==='vehicle'){
    const plate=normPlate(get('plate'));
    if(!get('clientId')||!findClient(get('clientId'))){toast('Buscá y seleccioná un cliente.','error');form.querySelector('#client-lookup')?.focus();return;}
    if(plate.length<5){toast('Ingresá una patente válida (al menos 5 caracteres).','error');return;}
    if(db.vehicles.some(v=>v.id!==id&&normPlate(v.plate)===plate)){toast('Ya existe un vehículo con esa patente.','error');return;}
    next={id:baseId,clientId:get('clientId'),plate,make:get('make'),model:get('model'),year:get('year')?Number(get('year')):'',mileage:amount(get('mileage')),vin:get('vin'),color:get('color'),nextServiceDate:get('nextServiceDate'),notes:get('notes'),createdAt:findVehicle(id)?.createdAt||isoDate(new Date())};
    routeTo=['vehiculo',baseId];message=existing?'Vehículo actualizado.':'Vehículo agregado.';
  }else if(kind==='order'){
    if(!get('vehicleId')||!findVehicle(get('vehicleId'))){toast('Primero buscá y seleccioná un vehículo de la lista.','error');form.querySelector('#vehicle-lookup')?.focus();return;}
    const price=amount(get('charged')),paid=amount(get('paid'));
    if(paid>price){toast('El total pagado no puede superar el importe acordado.','error');return;}
    if(!get('openedAt')){toast('Ingresá la fecha del trabajo.','error');return;}
    const previous=findOrder(id);
    const status=get('status');
    const wasClosed=previous&&['done','delivered'].includes(previous.status);
    next={id:baseId,vehicleId:get('vehicleId'),title:get('title'),category:get('category'),status,openedAt:get('openedAt'),completedAt:['done','delivered'].includes(status)?(wasClosed?previous.completedAt||isoDate(new Date()):isoDate(new Date())):'',mileage:amount(get('mileage')),description:get('description'),partsCost:amount(get('partsCost')),laborCost:amount(get('laborCost')),charged:price,paid,notes:get('notes'),createdAt:previous?.createdAt||isoDate(new Date())};
    routeTo=['trabajo',baseId];message=existing?'Trabajo actualizado.':'Trabajo registrado.';
  }else if(kind==='payment'){
    const o=findOrder(id);const payment=Number(get('payment'));
    if(!o||!Number.isFinite(payment)||payment<=0||payment>pendingAmount(o)+.00001){toast('Ingresá un importe válido, que no supere el saldo.','error');return;}
    next={...o,paid:amount(Number(o.paid)+payment)};routeTo=['trabajo',id];message='Cobro registrado.';
  }else return;
  const saved=await updateData(data=>{
    const key=kind==='client'?'clients':kind==='vehicle'?'vehicles':'orders';
    const index=data[key].findIndex(item=>item.id===baseId);
    if(index>=0)data[key][index]=next;else data[key].push(next);
    if(kind==='order'){
      const vehicle=data.vehicles.find(v=>v.id===next.vehicleId);
      if(vehicle&&next.mileage>vehicle.mileage)vehicle.mileage=next.mileage;
    }
  },message);
  if(saved){modalOriginal='';document.querySelector('#form-dialog').close();navigate(...routeTo);}
}
function confirmAction(title,message,acceptText='Eliminar'){
  return new Promise(resolve=>{
    const dialog=document.querySelector('#confirm-dialog');
    document.querySelector('#confirm-title').textContent=title;
    document.querySelector('#confirm-message').textContent=message;
    document.querySelector('#confirm-accept').textContent=acceptText;
    const onClose=()=>resolve(dialog.returnValue==='accept');
    dialog.addEventListener('close',onClose,{once:true});dialog.showModal();
  });
}
async function deleteRecord(kind,id){
  const key=kind==='client'?'clients':kind==='vehicle'?'vehicles':'orders';
  const item=db[key].find(x=>x.id===id);if(!item)return;
  if(kind==='client'&&db.vehicles.some(v=>v.clientId===id)){toast('No se puede eliminar un cliente que tenga vehículos.','error');return;}
  if(kind==='vehicle'&&db.orders.some(o=>o.vehicleId===id)){toast('No se puede eliminar un vehículo con historial de trabajos.','error');return;}
  if(!await confirmAction('Eliminar registro','Esta acción es irreversible si no tenés un respaldo.'))return;
  const saved=await updateData(d=>{d[key]=d[key].filter(x=>x.id!==id);},'Registro eliminado.');
  if(saved)navigate(kind==='client'?'clientes':kind==='vehicle'?'vehiculos':'trabajos');
}
function saveAsFile(content,filename,mime){
  const blob=new Blob([content],{type:mime});const url=URL.createObjectURL(blob);
  const a=document.createElement('a');a.href=url;a.download=filename;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),5000);
}
async function createDesktopBackup(silent=false){
  if(!desktopInvoke || desktopBackupBusy)return false;
  desktopBackupBusy=true;
  try{
    await desktopInvoke('backup_now');
    lastDesktopBackup=await desktopInvoke('backup_status');
    if(!silent)toast('Respaldo SQLite y Excel verificado en esta computadora. Copiá uno fuera del equipo.');
    if(route().page==='ajustes')render();
    return true;
  }catch(error){
    console.error('No se pudo completar el respaldo',error);
    toast(`Falló el respaldo: ${String(error)}`,'error');
    return false;
  }finally{desktopBackupBusy=false;}
}
async function importExcelDesktop(){
  if(!desktopInvoke)return;
  try{
    const preview=await desktopInvoke('choose_excel_import');
    if(!preview)return;
    const next=validateData(preview.data);
    const message=`Archivo: ${preview.filename}. Contiene ${next.clients.length} clientes, ${next.vehicles.length} vehículos y ${next.orders.length} trabajos. Se REEMPLAZARÁ todo lo actual. Se creará un respaldo previo local.`;
    if(!await confirmAction('Restaurar desde Excel',message,'Respaldar y reemplazar'))return;
    if(!await createDesktopBackup(true))throw Error('No se pudo crear un respaldo previo; la importación se canceló.');
    await saveData(next);
    navigate('inicio');toast('Excel importado con todos sus identificadores y relaciones.');
  }catch(e){console.error(e);toast(e?.message||String(e),'error');}
}
async function exportExcelDesktop(){
  if(!desktopInvoke)return;
  try{const wrote=await desktopInvoke('export_excel_dialog');if(wrote)toast('Excel completo exportado correctamente. Es confidencial.');}
  catch(e){console.error(e);toast(String(e),'error');}
}
async function scheduleDesktopBackups(){
  if(!desktopInvoke)return;
  try{
    lastDesktopBackup=await desktopInvoke('backup_status');
    const stale=!lastDesktopBackup.lastBackupUnixMs || Date.now()-Number(lastDesktopBackup.lastBackupUnixMs)>=4*60*60*1000;
    if(stale)await createDesktopBackup(true);
  }catch(e){console.error(e);toast('No se pudo comprobar el estado de las copias de seguridad','error');}
}
function exportJSON(){const backup={format:'autotrace-backup',version:1,exportedAt:new Date().toISOString(),data:db};saveAsFile(JSON.stringify(backup,null,2),`autotrace-respaldo-${isoDate(new Date())}.json`,'application/json;charset=utf-8');toast('Respaldo JSON descargado.');}
function exportCSV(){
  const headers=['Fecha','Estado','Categoría','Trabajo','Cliente','Teléfono','Patente','Vehículo','Kilometraje','Costo repuestos','Costo mano de obra','Importe','Pagado','Pendiente','Margen'];
  const rows=db.orders.map(o=>{const v=findVehicle(o.vehicleId),c=vehicleClient(v);return [o.openedAt,STATUSES[o.status],o.category,o.title,c?.name||'',c?.phone||'',v?.plate||'',fullVehicle(v),o.mileage,o.partsCost,o.laborCost,o.charged,o.paid,pendingAmount(o),margin(o)];});
  saveAsFile('\ufeff'+[headers,...rows].map(r=>r.map(csvCell).join(';')).join('\r\n'),`autotrace-trabajos-${isoDate(new Date())}.csv`,'text/csv;charset=utf-8');toast('CSV de trabajos descargado.');
}
async function importJSON(file){
  if(!file)return;
  try{
    if(file.size>5*1024*1024)throw Error('El archivo supera los 5 MB permitidos.');
    const payload=JSON.parse(await file.text());
    if(payload?.format && payload.format!=='autotrace-backup')throw Error('El respaldo no es de AutoTrace.');
    if(payload?.version && payload.version!==1)throw Error('La versión del respaldo no está soportada.');
    const next=validateData(payload?.data??payload);
    if(!await confirmAction('Reemplazar los datos actuales',`Se importarán ${next.clients.length} clientes, ${next.vehicles.length} vehículos y ${next.orders.length} trabajos. Se perderán los registros actuales si no los respaldaste.`, 'Importar y reemplazar'))return;
    if(desktopInvoke && !await createDesktopBackup(true))throw Error('No se pudo respaldar la información previa.');
    await saveData(next);navigate('inicio');toast('Respaldo restaurado correctamente.');
  }catch(e){console.error(e);toast(e?.message||'No se pudo importar el archivo.','error');}
  finally{document.querySelector('#import-file').value='';}
}
function printOrder(id){
  const order=findOrder(id);if(!order)return;
  const vehicle=findVehicle(order.vehicleId),client=vehicleClient(vehicle);
  const win=window.open('', '_blank');
  if(!win){toast('Permití las ventanas emergentes para imprimir.','error');return;}
  win.opener=null;
  // Las notas y los costos internos nunca se incluyen en un documento para el cliente.
  win.addEventListener('load',()=>{win.focus();win.print();},{once:true});
  win.document.write(buildCustomerSummary({order,vehicle,client,garageName:db.settings.garageName,currency:db.settings.currency}));
  win.document.close();
}
document.addEventListener('click',async event=>{
  const target=event.target.closest('[data-action],[data-nav],[data-filter],[data-list-filter],[data-list-page],[data-finance-filter],[data-history-filter],[data-history-page]');if(!target)return;
  if(target.dataset.nav){event.preventDefault();navigate(target.dataset.nav,target.dataset.id||'');return;}
  if(target.dataset.historyFilter){historyStatus=target.dataset.historyFilter;historyPage=1;refreshVehicleHistory();return;}
  if(target.dataset.historyPage){const page=Number(target.dataset.historyPage);if(!Number.isSafeInteger(page)||page<1)return;historyPage=page;refreshVehicleHistory();document.querySelector('.history-toolbar')?.scrollIntoView({block:'nearest',behavior:'instant'});return;}
  if(target.dataset.listFilter){const section=route().page;const key=target.dataset.listFilter;if(section==='clientes'&&['all','with','without'].includes(key))clientFilter=key;else if(section==='vehiculos'&&['all','due','overdue','soon','undated'].includes(key))vehicleFilter=key;else if(section==='trabajos'&&['all','active','closed','unpaid'].includes(key))activeFilter=key;else return;listPages[section]=1;refreshListResults();return;}
  if(target.dataset.financeFilter){financeFilter=target.dataset.financeFilter;financePage=1;refreshFinanceResults();return;}
  if(target.dataset.financePage){financePage=Number(target.dataset.financePage);refreshFinanceResults();return;}
  if(target.dataset.listPage){const section=route().page;if(section==='finanzas'){const requested=Number(target.dataset.listPage);if(!Number.isSafeInteger(requested)||requested<1)return;financePage=requested;refreshFinanceResults();document.querySelector('.finance-toolbar')?.scrollIntoView({block:'start',behavior:'instant'});return;}if(!Object.hasOwn(listPages,section))return;const requested=Number(target.dataset.listPage);if(!Number.isSafeInteger(requested)||requested<1)return;listPages[section]=requested;refreshListResults();document.querySelector('.list-tools')?.scrollIntoView({block:'start',behavior:'instant'});return;}
  if(target.dataset.filter){activeFilter=target.dataset.filter;refreshPage();return;}
  const action=target.dataset.action,id=target.dataset.id||'';
  switch(action){
    case 'go-home':navigate('inicio');break;
    case 'open-mobile-search':mobileSearchOpen=true;document.querySelector('.topbar')?.classList.add('search-open');document.querySelector('#global-search')?.focus();break;
    case 'close-mobile-search':activeQuery='';mobileSearchOpen=false;render();break;
    case 'go-settings':navigate('ajustes');break;
    case 'show-active-orders':activeFilter='active';listPages.trabajos=1;navigate('trabajos');break;
    case 'show-unpaid-orders':activeFilter='unpaid';listPages.trabajos=1;navigate('trabajos');break;
    case 'show-due-vehicles':vehicleFilter='due';listPages.vehiculos=1;navigate('vehiculos');break;
    case 'new-client':clientForm();break;
    case 'edit-client':clientForm(id);break;
    case 'view-client':navigate('cliente',id);break;
    case 'new-vehicle':vehicleForm('',target.dataset.client||'');break;
    case 'edit-vehicle':vehicleForm(id);break;
    case 'view-vehicle':navigate('vehiculo',id);break;
    case 'new-order':orderForm('',target.dataset.vehicle||'');break;
    case 'pick-vehicle':selectVehicleForOrder(id);break;
    case 'pick-client':selectClientForVehicle(id);break;
    case 'change-client':{
      const form=document.querySelector('#entity-form');if(form?.dataset.kind!=='vehicle')break;
      form.querySelector('#f-clientId').value='';form.querySelector('#client-selected').hidden=true;form.querySelector('.client-picker-search').hidden=false;
      form.querySelector('#client-lookup').value='';form.querySelector('#client-suggestions').innerHTML=clientPickerResults();form.querySelector('#client-lookup').focus();break;
    }
    case 'change-vehicle':{
      const form=document.querySelector('#entity-form');if(!form||form.dataset.kind!=='order')break;
      form.querySelector('#f-vehicleId').value='';form.querySelector('#vehicle-selected').hidden=true;form.querySelector('.vehicle-picker-search').hidden=false;
      form.querySelector('#vehicle-lookup').value='';form.querySelector('#vehicle-suggestions').innerHTML=vehiclePickerResults();form.querySelector('#vehicle-lookup').focus();break;
    }
    case 'clear-history':historyQuery='';historyPage=1;historyStatus='all';{const field=document.querySelector('#history-search');if(field){field.value='';field.focus();}}refreshVehicleHistory();break;
    case 'edit-order':orderForm(id);break;
    case 'add-payment':paymentForm(id);break;
    case 'print-order':printOrder(id);break;
    case 'delete-client':await deleteRecord('client',id);break;
    case 'delete-vehicle':await deleteRecord('vehicle',id);break;
    case 'delete-order':await deleteRecord('order',id);break;
    case 'close-modal':await closeFormSafely();break;
    case 'clear-search':activeQuery='';render();document.querySelector('#global-search')?.focus();break;
    case 'clear-list-search':{const section=route().page;if(!Object.hasOwn(listQueries,section))break;listQueries[section]='';listPages[section]=1;const input=document.querySelector('#list-search');if(input){input.value='';input.focus();}refreshListResults();break;}
    case 'clear-list-filters':{if(route().page==='finanzas'){financeQuery='';financePage=1;financeFilter='all';const field=document.querySelector('#finance-search');if(field){field.value='';field.focus();}refreshFinanceResults();break;}const section=route().page;if(!Object.hasOwn(listQueries,section))break;listQueries[section]='';listPages[section]=1;if(section==='clientes')clientFilter='all';if(section==='vehiculos')vehicleFilter='all';if(section==='trabajos')activeFilter='all';const input=document.querySelector('#list-search');if(input){input.value='';input.focus();}refreshListResults();break;}
    case 'export-json':exportJSON();break;
    case 'export-excel-desktop':await exportExcelDesktop();break;
    case 'export-client-excel':if(desktopInvoke && id){try{if(await desktopInvoke('export_customer_excel_dialog',{clientId:id}))toast('Informe Excel exportado sin costos internos ni notas privadas.');}catch(error){toast(String(error),'error');}}break;
    case 'import-excel-desktop':await importExcelDesktop();break;
    case 'backup-desktop-now':await createDesktopBackup();break;
    case 'export-csv':exportCSV();break;
    case 'import-json':document.querySelector('#import-file').click();break;
    case 'reset-demo':{
      if(!await confirmAction('Comenzar sin datos de muestra','Esto eliminará TODOS los registros, incluidos los que hayas agregado durante la demostración. Exportá un respaldo primero si querés conservarlos.','Borrar y comenzar'))break;
      if(desktopInvoke && !await createDesktopBackup(true))break;
      const saved=await updateData(d=>{Object.assign(d,canonical());},'Ahora tenés un espacio vacío para tu taller.');if(saved)navigate('inicio');break;
    }
    case 'clear-all':{
      if(!await confirmAction('Eliminar absolutamente todo','Se borrarán clientes, vehículos, servicios, importes y configuración de este navegador. No se puede deshacer sin un respaldo.','Borrar todos los datos'))break;
      if(desktopInvoke && !await createDesktopBackup(true))break;
      const saved=await updateData(d=>{Object.assign(d,canonical());},'Todos los datos fueron eliminados.');if(saved)navigate('inicio');break;
    }
  }
});
document.addEventListener('submit',async event=>{
  if(event.target.id==='entity-form'){event.preventDefault();await saveForm(event.target);}
  if(event.target.id==='settings-form'){
    event.preventDefault();const fd=new FormData(event.target);const name=String(fd.get('garageName')||'').trim();const currency=String(fd.get('currency')||'');
    if(!name||!['ARS','USD','EUR'].includes(currency)){toast('Verificá las preferencias.','error');return;}
    await updateData(d=>{d.settings.garageName=name;d.settings.currency=currency;},'Preferencias guardadas.');
  }
});
document.addEventListener('change',async event=>{
  if(event.target.id==='import-file')await importJSON(event.target.files?.[0]);
  if(event.target.dataset.statusOrder){
    const id=event.target.dataset.statusOrder,status=event.target.value;
    if(!STATUSES[status])return;
    await updateData(d=>{const o=d.orders.find(x=>x.id===id);if(!o)return;const wasClosed=['done','delivered'].includes(o.status);o.status=status;o.completedAt=['done','delivered'].includes(status)?(wasClosed?o.completedAt||isoDate(new Date()):isoDate(new Date())):'';},'Estado actualizado.');
  }
});
document.addEventListener('input',event=>{
  if(event.target.id==='global-search'){activeQuery=event.target.value;refreshPage();}
  if(event.target.id==='client-lookup'){const results=document.querySelector('#client-suggestions');if(results)results.innerHTML=clientPickerResults(event.target.value);}
  if(event.target.id==='vehicle-lookup'){const results=document.querySelector('#vehicle-suggestions');if(results)results.innerHTML=vehiclePickerResults(event.target.value);}
  if(event.target.id==='history-search'){historyQuery=event.target.value;historyPage=1;refreshVehicleHistory();}
  if(event.target.id==='finance-search'){financeQuery=event.target.value;financePage=1;refreshFinanceResults();}
  if(event.target.dataset.listSearch){const section=event.target.dataset.listSearch;if(!Object.hasOwn(listQueries,section))return;listQueries[section]=event.target.value;listPages[section]=1;refreshListResults();}
});
async function closeFormSafely(){
  const dialog=document.querySelector('#form-dialog');const form=dialog.querySelector('#entity-form');
  if(!form || modalOriginal===modalSnapshot(form)){modalOriginal='';dialog.close();return;}
  if(await confirmAction('¿Descartar los cambios?','Modificaste datos del formulario que todavía no se guardaron.','Descartar cambios')){modalOriginal='';dialog.close();}
}
document.querySelector('#form-dialog').addEventListener('cancel',event=>{event.preventDefault();void closeFormSafely();});
document.addEventListener('keydown',event=>{
  const typing=['INPUT','TEXTAREA','SELECT'].includes(document.activeElement?.tagName) || document.activeElement?.isContentEditable;
  if((event.key==='/'&&!typing || (event.key.toLowerCase()==='k'&&(event.ctrlKey||event.metaKey)))&&!document.querySelector('dialog[open]')){
    event.preventDefault();document.querySelector('#global-search')?.focus();
  }
  if(event.key==='Escape' && document.activeElement?.id==='global-search' && activeQuery){activeQuery='';render();document.querySelector('#global-search')?.focus();}
});
window.addEventListener('hashchange' ,()=>{activeQuery='';render();});
if(!desktopInvoke && 'serviceWorker' in navigator&&(location.protocol==='https:'||location.hostname==='localhost')){
  window.addEventListener('load',()=>navigator.serviceWorker.register('./sw.js').catch(err=>console.warn('PWA sin service worker:',err)));
}
try {
  await loadData();render();
if(desktopInvoke){
  await scheduleDesktopBackups();
  setInterval(scheduleDesktopBackups,4*60*60*1000);
}
} catch(err) {
  console.error('No se pudo abrir la base de AutoTrace:',err);
  app.textContent='No se pudo abrir la base de datos de AutoTrace. No se realizaron cambios. Revisá los permisos y la versión de la aplicación antes de continuar.';
}
