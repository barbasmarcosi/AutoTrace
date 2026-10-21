/** Sugerencias acotadas para formularios: sin listas interminables ni scroll anidado. */
import {matchesClient, matchesVehicle, normalizeSearch} from './search.mjs';

export const DEFAULT_SUGGESTIONS = 3;
export const SEARCH_SUGGESTIONS = 5;

/** Las últimas visitas ganan prioridad cuando todavía no se escribió una búsqueda. */
export function suggestVehicles(vehicles, clients, orders, query = '') {
  const clientById = new Map(clients.map(client => [client.id, client]));
  const recentVisit = new Map();
  for (const order of orders) {
    const date = order.openedAt || order.createdAt || '';
    if (date > (recentVisit.get(order.vehicleId) || '')) recentVisit.set(order.vehicleId, date);
  }
  const searching = !!normalizeSearch(query);
  const matched = vehicles.filter(vehicle => matchesVehicle(vehicle, clientById.get(vehicle.clientId), query));
  matched.sort((a, b) => searching
    ? a.plate.localeCompare(b.plate, 'es')
    : (recentVisit.get(b.id) || '').localeCompare(recentVisit.get(a.id) || '') ||
      (b.createdAt || '').localeCompare(a.createdAt || '') || a.plate.localeCompare(b.plate, 'es'));
  const limit = searching ? SEARCH_SUGGESTIONS : DEFAULT_SUGGESTIONS;
  return {items: matched.slice(0, limit), total: matched.length, limit, searching};
}

export function suggestClients(clients, query = '') {
  const searching = !!normalizeSearch(query);
  const matched = clients.filter(client => matchesClient(client, query));
  matched.sort((a, b) => searching
    ? a.name.localeCompare(b.name, 'es')
    : (b.createdAt || '').localeCompare(a.createdAt || '') || a.name.localeCompare(b.name, 'es'));
  const limit = searching ? SEARCH_SUGGESTIONS : DEFAULT_SUGGESTIONS;
  return {items: matched.slice(0, limit), total: matched.length, limit, searching};
}
