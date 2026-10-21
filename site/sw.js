// Mantener separados los cachés de AutoTrace: otros proyectos de GitHub Pages
// comparten el mismo origen, pero no deben verse afectados por esta PWA.
const CACHE = 'autotrace-shell-v7';
const PREFIX = 'autotrace-shell-';
const ASSETS = ['./', './index.html', './styles.css', './app.js', './model.mjs', './search.mjs', './pickers.mjs', './customer-summary.mjs', './manifest.webmanifest', './assets/icon.svg', './assets/icon-192.png', './assets/icon-512.png', './assets/apple-touch-icon.png'];
const scope = self.registration.scope;
const shellPaths = new Set(ASSETS.map(asset => new URL(asset, scope).pathname));
self.addEventListener('install', event => {
  event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(ASSETS)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', event => {
  event.waitUntil(Promise.all([
    caches.keys().then(keys => Promise.all(keys.filter(key => key.startsWith(PREFIX) && key !== CACHE).map(key => caches.delete(key)))),
    self.clients.claim()
  ]));
});
self.addEventListener('fetch', event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== 'GET' || url.origin !== self.location.origin || !url.pathname.startsWith(new URL(scope).pathname)) return;
  const isNavigation = request.mode === 'navigate';
  const isShellAsset = shellPaths.has(url.pathname);
  if (!isNavigation && !isShellAsset) return;
  event.respondWith((async () => {
    try {
      const response = await fetch(request);
      if (response.ok && (isNavigation || isShellAsset)) {
        const cache = await caches.open(CACHE);
        await cache.put(request, response.clone());
      }
      return response;
    } catch {
      return (await caches.match(request)) || (isNavigation ? await caches.match('./index.html') : Response.error());
    }
  })());
});
