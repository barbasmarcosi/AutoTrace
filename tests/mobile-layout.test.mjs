import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const css=readFileSync(new URL('../site/styles.css',import.meta.url),'utf8');
const app=readFileSync(new URL('../site/app.js',import.meta.url),'utf8');

// Contratos de layout respaldados también por pruebas de navegador a 320–1440 px.
test('modal móvil: un solo eje de desplazamiento y grilla que puede achicarse',()=>{
  assert.match(css,/#form-dialog \.modal-body\s*\{[^}]*overflow-x:\s*hidden;[^}]*overflow-y:\s*auto;[^}]*touch-action:\s*pan-y;/s);
  assert.match(css,/#form-dialog \.form-grid\s*\{\s*grid-template-columns:\s*minmax\(0,\s*1fr\);/);
  assert.match(css,/#form-dialog \.field input,[\s\S]*?#form-dialog \.field-full textarea\s*\{[^}]*min-width:\s*0;/);
});

test('alta rápida: etiqueta accesible y símbolo + centrado en móvil',()=>{
  assert.match(app,/topbar-create-label/);
  assert.match(app,/aria-label="Nuevo trabajo"/);
  assert.match(css,/\.topbar \.topbar-create\s*\{[^}]*place-items:\s*center;[^}]*gap:\s*0;/);
  assert.match(css,/\.topbar \.topbar-create-label\s*\{\s*display:\s*none;/);
});
