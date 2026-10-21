import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {buildCustomerSummary} from '../site/customer-summary.mjs';

const fixture = () => ({
  order: { title: 'Revisión de <frenos>', status: 'done', openedAt: '2026-10-20', mileage: 95000,
    description: 'Reemplazo de pastillas delanteras.', notes: 'NOTA_INTERNA_NO_IMPRIMIR_123',
    partsCost: 7777777, laborCost: 8888888, charged: 200000, paid: 50000 },
  vehicle: { make: 'Ford', model: 'Fiesta', plate: 'AF382QL' },
  client: {name: 'Andrea López', phone: '2235550010'},
  garageName: 'Taller & familia', currency: 'ARS'
});
test('resumen para clientes no divulga costos ni notas internas',()=>{
  const html=buildCustomerSummary(fixture());
  assert.match(html,/Reemplazo de pastillas delanteras/);
  assert.match(html,/Saldo pendiente/);
  assert.match(html,/150\.000/);
  assert.match(html,/AF382QL/);
  assert.doesNotMatch(html,/NOTA_INTERNA_NO_IMPRIMIR_123|7777777|8888888|Costos internos|Margen/);
});
test('el resumen escapa entradas de usuarios sin interpolar HTML activo',()=>{
  const html=buildCustomerSummary(fixture());
  assert.match(html,/Revisión de &lt;frenos&gt;/);
  assert.match(html,/Taller &amp; familia/);
  assert.doesNotMatch(html,/<frenos>/);
});
test('el delegador admite filtros y paginación del historial del vehículo',()=>{
  const code=readFileSync(new URL('../site/app.js', import.meta.url),'utf8');
  const match=code.match(/event\.target\.closest\('([^']+)'\)/);
  assert.ok(match,'selector delegado presente');
  assert.ok(match[1].includes('[data-history-filter]'));
  assert.ok(match[1].includes('[data-history-page]'));
});

test('los tres indicadores del inicio abren listados filtrados',()=>{
  const code=readFileSync(new URL('../site/app.js',import.meta.url),'utf8');
  for(const action of ['show-active-orders','show-unpaid-orders','show-due-vehicles']){
    assert.match(code,new RegExp(`data-action="${action}"`));
    assert.match(code,new RegExp(`case '${action}':`));
  }
});
