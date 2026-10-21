import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

function makeWorker(keys) {
  const listeners={};const removed=[];
  const cache={addAll:async()=>{},put:async()=>{}};
  const context={
    self: {registration:{scope:'https://example.com/AutoTrace/'},location:{origin:'https://example.com'},
      clients:{claim:async()=>{}},skipWaiting:async()=>{},
      addEventListener:(kind,fn)=>listeners[kind]=fn},
    URL, Set, Response,
    caches: {open:async()=>cache,keys:async()=>keys,delete:async key=>{removed.push(key);return true},match:async()=>undefined},
    fetch: async()=>({ok:true,clone(){return this;}})
  };
  vm.runInNewContext(readFileSync(new URL('../site/sw.js',import.meta.url),'utf8'),context);
  return {listeners,removed};
}
test('la actualización no borra cachés de otras aplicaciones',async()=>{
  const {listeners,removed}=makeWorker(['autotrace-shell-v3','autotrace-shell-v5','otra-pwa-v1']);
  let work;listeners.activate({waitUntil:p=>work=p});await work;
  assert.deepEqual(removed,['autotrace-shell-v3']);
});
test('el SW atiende solo rutas y recursos de AutoTrace',()=>{
  const {listeners}=makeWorker([]);
  const doesIntercept=(url,mode='same-origin')=>{
    let intercepted=false;
    listeners.fetch({request:{url,method:'GET',mode},respondWith:()=>intercepted=true});
    return intercepted;
  };
  assert.equal(doesIntercept('https://example.com/AutoTrace/app.js'),true);
  assert.equal(doesIntercept('https://example.com/AutoTrace/customer-summary.mjs'),true);
  assert.equal(doesIntercept('https://example.com/AutoTrace/'),true);
  assert.equal(doesIntercept('https://example.com/AutoTrace/clientes','#not-navigate'),false);
  assert.equal(doesIntercept('https://example.com/OtroProyecto/app.js'),false);
  assert.equal(doesIntercept('https://example.com/AutoTrace/usuario.json'),false);
  assert.equal(doesIntercept('https://example.net/AutoTrace/app.js'),false);
});
