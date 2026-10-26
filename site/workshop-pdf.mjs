/** Offline customer-facing A4 PDF export. No network, third-party service,
 * internal costs, workshop notes, payment methods, or other customers.
 *
 * Small fixed-layout PDF 1.4 writer for Latin/Spanish workshop documents.
 * Helvetica/Helvetica-Bold are standard PDF fonts. Unsupported glyphs cause
 * a clear error rather than silently changing a person's name.
 */
import {validateData} from './model.mjs';

const PAGE_W=595.28,PAGE_H=841.89,M=43,RIGHT=PAGE_W-M;
const NAVY=[.06,.14,.18],INK=[.10,.16,.20],GRAY=[.40,.46,.50],BLUE=[.10,.39,.50];
const WINANSI={0x20ac:128,0x201a:130,0x0192:131,0x201e:132,0x2026:133,0x2020:134,
  0x2021:135,0x02c6:136,0x2030:137,0x0160:138,0x2039:139,0x0152:140,0x017d:142,
  0x2018:145,0x2019:146,0x201c:147,0x201d:148,0x2022:149,0x2013:150,0x2014:151,
  0x02dc:152,0x2122:153,0x0161:154,0x203a:155,0x0153:156,0x017e:158,0x0178:159};
function ansi(text){
  let s='';
  for(const c of String(text??'')){
    const n=c.codePointAt(0),mapped=n<=255&&!(n>=127&&n<=159)?n:WINANSI[n];
    if(mapped===undefined||mapped<32&&mapped!==9&&mapped!==10&&mapped!==13)
      throw Error('El PDF contiene un carácter no compatible con la tipografía estándar. Usá "Resumen para cliente" > Guardar como PDF para este documento.');
    s+=String.fromCharCode(mapped);
  }
  return s;
}
function lit(value){
  const s=ansi(value).replace(/[\r\n\t]+/g,' ').replace(/\\/g,'\\\\').replace(/\(/g,'\\(').replace(/\)/g,'\\)');
  return '('+s+')';
}
function rgb(v){return v.map(n=>n.toFixed(3)).join(' ');}
function sizeEstimate(s,font=10){return [...String(s)].length*font*.53;}
function wrap(value,maxWidth,font=10){
  // Limit lines by conservative font width; break long unspaced references
  // and retain manual line breaks in customer-facing part descriptions.
  const limit=Math.max(1,Math.floor(maxWidth/(font*.59)));
  const out=[];
  for(const paragraph of String(value??'').split(/\r?\n/)){
    const words=paragraph.trim().split(/\s+/).filter(Boolean);
    if(!words.length){out.push('');continue;}
    let line='';
    for(let word of words){
      while(word.length>limit){
        if(line){out.push(line);line='';}
        out.push(word.slice(0,limit));word=word.slice(limit);
      }
      if(!word)continue;
      if(!line)line=word;
      else if((line+' '+word).length<=limit)line+=' '+word;
      else{out.push(line);line=word;}
    }
    if(line)out.push(line);
  }
  return out.length?out:[''];
}
function formatMoney(value,currency){
  const formatted=Number(value).toLocaleString('es-AR',{minimumFractionDigits:2,maximumFractionDigits:2});
  return (currency==='ARS'?'$ ':currency+' ')+formatted;
}
function moneyCents(value){
  if(!Number.isFinite(value)||value<0||value>1e12)throw Error('Importe inválido para PDF');
  return Math.round(value*100);
}
function binary(bytes){return Uint8Array.from(bytes,c=>c.charCodeAt(0));}
function pdfObjects(pages){
  // Objects 1/2: catalog/pages; 3/4: the standard built-in typefaces;
  // each page uses one Page and one Contents stream.
  const objects=[
    '<< /Type /Catalog /Pages 2 0 R >>','', 
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>',
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>'
  ];
  const kids=[];
  for(const ops of pages){
    const pageNo=objects.length+1,contentNo=pageNo+1;
    kids.push(pageNo+' 0 R');
    objects.push('<< /Type /Page /Parent 2 0 R /MediaBox [0 0 '+PAGE_W+' '+PAGE_H+
      '] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents '+contentNo+' 0 R >>');
    const stream=ops.join('\n')+'\n';
    objects.push('<< /Length '+stream.length+' >>\nstream\n'+stream+'endstream');
  }
  objects[1]='<< /Type /Pages /Kids ['+kids.join(' ')+'] /Count '+pages.length+' >>';
  let pdf='%PDF-1.4\n%\xE2\xE3\xCF\xD3\n';
  const offsets=[0];
  for(let i=0;i<objects.length;i++){
    offsets.push(pdf.length);
    pdf+=(i+1)+' 0 obj\n'+objects[i]+'\nendobj\n';
  }
  const startxref=pdf.length;
  pdf+='xref\n0 '+(objects.length+1)+'\n0000000000 65535 f \n';
  for(const offset of offsets.slice(1))pdf+=String(offset).padStart(10,'0')+' 00000 n \n';
  pdf+='trailer\n<< /Size '+(objects.length+1)+' /Root 1 0 R >>\nstartxref\n'+startxref+'\n%%EOF\n';
  return binary(pdf);
}
export function writeWorkshopPdf(data,orderId,kind){
  if(!['quote','work'].includes(kind))throw Error('Elegí presupuesto o remito.');
  const state=validateData(data);
  const order=state.orders.find(o=>o.id===orderId);
  if(!order)throw Error('Trabajo inexistente.');
  if(!order.items?.length)throw Error('Cargá el detalle de piezas y tareas antes de exportar.');
  const car=state.vehicles.find(v=>v.id===order.vehicleId);
  const client=state.clients.find(c=>c.id===car?.clientId);
  if(!car||!client)throw Error('Faltan los datos del vehículo o cliente.');
  const sum=order.items.reduce((total,x)=>total+Math.round(x.quantity*x.unitPrice*100),0);
  if(Math.abs(sum-moneyCents(order.charged))>1)throw Error('El importe acordado y el detalle no coinciden.');
  const currency=state.settings.currency||'ARS',label=kind==='quote'?'PRESUPUESTO':'REMITO DE TRABAJO';
  const pages=[];
  let ops,y;
  function fill(color){ops.push(rgb(color)+' rg');}
  function rect(x,top,w,h,color){fill(color);ops.push(x+' '+(PAGE_H-top-h)+' '+w+' '+h+' re f');}
  function line(x1,top1,x2,top2,color=GRAY){ops.push(rgb(color)+' RG 0.6 w '+x1+' '+(PAGE_H-top1)+' m '+x2+' '+(PAGE_H-top2)+' l S');}
  function text(value,x,top,font=10,{bold=false,color=INK,right=false}={}){
    let displayed=String(value??'');
    // Encode before alignment to ensure an unsupported glyph fails *before*
    // the final file can be returned to the operator.
    ansi(displayed);
    if(right)x-=sizeEstimate(displayed,font);
    fill(color);
    ops.push('BT /'+(bold?'F2':'F1')+' '+font+' Tf 1 0 0 1 '+x.toFixed(2)+' '+(PAGE_H-top).toFixed(2)+' Tm '+lit(displayed)+' Tj ET');
  }
  function page(){
    ops=[];pages.push(ops);
    rect(0,0,PAGE_W,83,NAVY);
    text(label,M,41,21,{bold:true,color:[1,1,1]});
    const workshop=wrap(state.settings.garageName||'Taller',230,10).slice(0,2);
    workshop.forEach((s,i)=>text(s,RIGHT,31+i*15,10,{bold:true,color:[1,1,1],right:true}));
    y=112;
  }
  function head(){
    text('FECHA',M,y,8,{bold:true,color:GRAY});
    text(order.openedAt,M,y+17,11,{bold:true});
    text('DOCUMENTO',345,y,8,{bold:true,color:GRAY});
    text(order.documentNumber||'Sin numeración',345,y+17,11,{bold:true});
    y+=51;line(M,y,RIGHT,y);y+=25;
    text('CLIENTE',M,y,9,{bold:true,color:BLUE});
    y+=19;
    for(const s of wrap(client.name,490,12)){text(s,M,y,12,{bold:true});y+=16;}
    if(client.phone){text('Teléfono: '+client.phone,M,y+4,10);y+=16;}
    if(client.email){for(const s of wrap('Correo: '+client.email,490,9)){text(s,M,y+4,9);y+=14;}}
    y+=14;
    line(M,y,RIGHT,y);y+=22;
    text('VEHÍCULO',M,y,9,{bold:true,color:BLUE});
    y+=19;
    for(const s of wrap([car.make,car.model].filter(Boolean).join(' '),490,11)){text(s,M,y,11,{bold:true});y+=16;}
    text('Patente: '+(car.plate||'No informada'),M,y+4,10);
    text('Kilometraje: '+Number(order.mileage||0).toLocaleString('es-AR')+' km',345,y+4,10);
    y+=36;
    line(M,y,RIGHT,y);y+=22;
  }
  function tableHead(){
    rect(M,y-11,RIGHT-M,26,[.91,.94,.96]);
    text('DETALLE DE TRABAJOS Y PIEZAS',M+9,y+6,9,{bold:true});
    text('CANT.',367,y+6,9,{bold:true});
    text('UNITARIO',443,y+6,9,{bold:true,right:true});
    text('TOTAL',RIGHT-8,y+6,9,{bold:true,right:true});
    y+=34;
  }
  function nextPage(){
    page();
    text('CONTINUACIÓN',M,y+3,10,{bold:true,color:GRAY});
    y+=35;
    tableHead();
  }
  page();
  head();
  tableHead();
  for(const item of order.items){
    const description=wrap(item.description,284,9.5);
    const height=Math.max(34,description.length*13+15);
    if(height>PAGE_H-170)throw Error('Una descripción del documento excede una página PDF.');
    if(y+height>PAGE_H-112)nextPage();
    const baseline=y+5;
    description.forEach((s,i)=>text(s,M+8,baseline+i*13,9.5));
    text(Number(item.quantity).toLocaleString('es-AR',{maximumFractionDigits:3}),368,baseline,9,{right:false});
    text(formatMoney(item.unitPrice,currency),460,baseline,9,{right:true});
    text(formatMoney(Math.round(item.quantity*item.unitPrice*100)/100,currency),RIGHT-8,baseline,9,{right:true});
    y+=height;line(M,y-10,RIGHT,y-10,[.83,.86,.88]);
  }
  if(y+106>PAGE_H-85)nextPage();
  y+=13;
  rect(305,y-10,RIGHT-305,42,[.91,.94,.96]);
  text('TOTAL',317,y+14,11,{bold:true});
  text(formatMoney(sum/100,currency),RIGHT-8,y+14,13,{bold:true,right:true});
  y+=76;
  for(const s of wrap(kind==='quote'?'Presupuesto sujeto a revisión y aprobación.':
    'Detalle de trabajos y piezas informados al cliente.',RIGHT-M,9)){
    text(s,M,y,9,{color:GRAY});y+=14;
  }
  for(let i=0;i<pages.length;i++){
    ops=pages[i];
    line(M,PAGE_H-60,RIGHT,PAGE_H-60,[.80,.85,.88]);
    text('AutoTrace · Documento informativo, no válido como factura fiscal',M,PAGE_H-42,8,{color:GRAY});
    text('Página '+(i+1)+' de '+pages.length,RIGHT,PAGE_H-42,8,{color:GRAY,right:true});
  }
  const bytes=pdfObjects(pages);
  if(bytes.length>5_000_000)throw Error('El documento PDF excede el tamaño permitido.');
  return bytes;
}
