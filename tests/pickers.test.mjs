import test from 'node:test';
import assert from 'node:assert/strict';
import {suggestVehicles, suggestClients, DEFAULT_SUGGESTIONS, SEARCH_SUGGESTIONS} from '../site/pickers.mjs';
const clients = Array.from({length: 50}, (_, i) => ({id:`c${i}`, name:`Cliente ${String(i).padStart(2,'0')}`, createdAt:`2026-10-${String(i%28+1).padStart(2,'0')}`}));
const vehicles = clients.map((c, i) => ({id:`v${i}`, clientId:c.id, plate:`AF${String(i).padStart(5,'0')}`, make:'Toyota', model:'Corolla', createdAt:`2026-10-${String(i%28+1).padStart(2,'0')}`}));

test('the initial vehicle picker has a hard cap of three and prioritizes recently serviced vehicles', () => {
  const r = suggestVehicles(vehicles, clients, [{vehicleId:'v3',openedAt:'2026-10-19'}, {vehicleId:'v4',openedAt:'2026-10-21'}]);
  assert.equal(r.total,50);assert.equal(r.items.length,DEFAULT_SUGGESTIONS);assert.equal(r.items[0].id,'v4');assert.equal(r.items[1].id,'v3');
});
test('vehicle search includes owner, ignores plate separators, limits to five', () => {
  let r=suggestVehicles(vehicles,clients,[],'Cliente 04');
  assert.equal(r.total,11);assert.equal(r.items.length,SEARCH_SUGGESTIONS);
  r=suggestVehicles(vehicles,clients,[],'AF-00042');assert.equal(r.items.length,1);assert.equal(r.items[0].id,'v42');
});
test('client suggestions have a hard cap and are searchable', () => {
  assert.equal(suggestClients(clients).items.length,3);
  assert.equal(suggestClients(clients,'Cliente').items.length,5);
  assert.equal(suggestClients(clients,'Cliente 17').items[0].id,'c17');
});
test('empty matches result in zero results and no extra items', () => {
  assert.deepEqual(suggestClients(clients,'no existe').items,[]);
  assert.deepEqual(suggestVehicles(vehicles,clients,[],'XYZZ').items,[]);
});
