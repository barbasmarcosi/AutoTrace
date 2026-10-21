/** Documento exclusivamente para entregar al cliente. Nunca incluir costos internos ni notas privadas. */
const escapeHTML = value => String(value ?? '').replace(/[&<>"']/g, char => ({
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
})[char]);
const currencyFormat = (value, currency) => new Intl.NumberFormat('es-AR', {
  style: 'currency', currency: ['ARS', 'USD', 'EUR'].includes(currency) ? currency : 'ARS',
  minimumFractionDigits: 0, maximumFractionDigits: 2
}).format(Number(value) || 0);
const dateFormat = value => /^\d{4}-\d{2}-\d{2}$/.test(value ?? '')
  ? new Date(`${value}T12:00:00`).toLocaleDateString('es-AR', {day: '2-digit', month: 'long', year: 'numeric'}) : 'Sin fecha';

export function buildCustomerSummary({order, vehicle, client, garageName = 'Mi taller', currency = 'ARS'}) {
  const o = order;
  const due = Math.max(0, Math.round(((Number(o.charged) || 0) - (Number(o.paid) || 0)) * 100) / 100);
  const statusNames = {received: 'Recibido', progress: 'En proceso', waiting: 'Esperando repuesto', done: 'Finalizado', delivered: 'Entregado'};
  const brand = escapeHTML(garageName);
  const lines = [
    ['Cliente', client?.name || 'Sin datos'],
    ['Vehículo', vehicle ? `${vehicle.make} ${vehicle.model}` : 'Sin datos'],
    ['Patente', vehicle?.plate || 'Sin datos'],
    ['Kilometraje registrado', `${new Intl.NumberFormat('es-AR').format(Number(o.mileage)||0)} km`]
  ];
  return `<!doctype html>
<html lang="es"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Resumen de servicio · ${escapeHTML(o.title)}</title>
<style>
  :root{color-scheme:light}*{box-sizing:border-box}body{font-family:system-ui,-apple-system,"Segoe UI",sans-serif;max-width:800px;margin:0 auto;padding:48px 34px;background:#fff;color:#20363a;font-size:14px;line-height:1.6}
  header{display:flex;justify-content:space-between;gap:20px;align-items:flex-start;padding-bottom:24px;border-bottom:3px solid #16836a}
  .brand{font-size:23px;font-weight:850;letter-spacing:-.04em}.overline{font-size:11px;color:#52716e;text-transform:uppercase;letter-spacing:.12em;font-weight:750}
  h1{font-size:25px;line-height:1.22;margin:26px 0 9px;overflow-wrap:anywhere}h2{font-size:15px;margin:30px 0 14px;border-bottom:1px solid #dde7e3;padding-bottom:9px}
  .meta{color:#657d7c;font-size:12px}.grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}
  .field{min-width:0}.field small{display:block;font-size:11px;color:#6a807f;margin-bottom:4px}.field strong{display:block;font-size:14px;overflow-wrap:anywhere}
  .description{white-space:pre-wrap;overflow-wrap:anywhere;margin:0}
  .totals{background:#f0f8f5;border:1px solid #d9eae3;border-radius:13px;padding:20px;display:grid;gap:12px}
  .total-line{display:flex;justify-content:space-between;align-items:baseline;gap:20px}.total-line strong{font-size:17px}
  .balance{border-top:1px solid #c6dfd5;padding-top:12px;font-weight:750}
  footer{margin-top:40px;border-top:1px solid #e1e8e4;padding-top:16px;font-size:11px;color:#69817e}
  @media(max-width:530px){body{padding:24px 18px}.grid{grid-template-columns:1fr}.brand{font-size:19px}h1{font-size:22px}}
  @media print{body{max-width:none;padding:0}header{break-after:avoid}.totals{print-color-adjust:exact;-webkit-print-color-adjust:exact}}
</style></head><body>
<header><div><div class="overline">AutoTrace · Servicio automotor</div><div class="brand">${brand}</div></div><div class="overline">Resumen de servicio</div></header>
<h1>${escapeHTML(o.title)}</h1>
<p class="meta">Ingreso: ${escapeHTML(dateFormat(o.openedAt))} · Estado: ${escapeHTML(statusNames[o.status] || o.status || 'Sin estado')}</p>
<h2>Datos del vehículo y cliente</h2><div class="grid">${lines.map(([label, value]) => `<div class="field"><small>${escapeHTML(label)}</small><strong>${escapeHTML(value)}</strong></div>`).join('')}</div>
<h2>Servicio y diagnóstico</h2><p class="description">${escapeHTML(o.description || 'Sin detalle adicional.')}</p>
<h2>Resumen de importes</h2><div class="totals">
<div class="total-line"><span>Importe acordado</span><strong>${escapeHTML(currencyFormat(o.charged, currency))}</strong></div>
<div class="total-line"><span>Total abonado</span><strong>${escapeHTML(currencyFormat(o.paid, currency))}</strong></div>
<div class="total-line balance"><span>Saldo pendiente</span><strong>${escapeHTML(currencyFormat(due, currency))}</strong></div>
</div>
<footer>Documento informativo del trabajo registrado. No es una factura ni un comprobante fiscal. Los importes son los cargados en AutoTrace al momento de imprimir.</footer>
</body></html>`;
}
