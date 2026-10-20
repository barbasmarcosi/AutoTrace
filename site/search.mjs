/** Búsqueda local, tolerante a tildes, espacios y separadores de patente. */
export const normalizeSearch = value => String(value ?? '').normalize('NFD')
  .replace(/[\u0300-\u036f]/g, '').toLocaleLowerCase('es').trim().replace(/\s+/g, ' ');
const compact = value => normalizeSearch(value).replace(/[^a-z0-9]/g, '');

export function matchesFields(query, fields) {
  const terms = normalizeSearch(query).split(/\s+/).filter(Boolean);
  if (!terms.length) return true;
  const values = fields.map(value => ({ normal: normalizeSearch(value), compact: compact(value) }));
  return terms.every(term => values.some(v => v.normal.includes(term) || (compact(term) !== '' && v.compact.includes(compact(term)))));
}

export const matchesClient = (client, query) => matchesFields(query,
  [client.name, client.phone, client.email, client.notes]);
export const matchesVehicle = (vehicle, client, query) => matchesFields(query,
  [vehicle.plate, vehicle.make, vehicle.model, vehicle.year, vehicle.vin,
    vehicle.color, vehicle.notes, client?.name, client?.phone]);
export const matchesOrder = (order, vehicle, client, query) => matchesFields(query,
  [order.title, order.category, order.description, order.notes, order.status,
    vehicle?.plate, vehicle?.make, vehicle?.model, client?.name, client?.phone]);

export function paginate(items, requestedPage, pageSize = 12) {
  const total = items.length;
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const page = Math.max(1, Math.min(pages, Number(requestedPage) || 1));
  const start = (page - 1) * pageSize;
  return {items: items.slice(start, start + pageSize), total, page, pages,
    first: total ? start + 1 : 0, last: Math.min(total, start + pageSize)};
}
