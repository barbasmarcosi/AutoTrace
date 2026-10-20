# AutoTrace

PWA de gestión liviana para talleres automotrices. Clientes, vehículos, intervenciones, vencimientos de servicios y balance básico de costos/cobros. **Versión local-first, sin backend.**

## Aplicación

Al publicarse por GitHub Pages: <https://barbasmarcosi.github.io/AutoTrace/>

## Qué incluye

- Alta, consulta, edición y baja segura de clientes, vehículos y trabajos.
- Relaciones cliente → vehículos → historial de trabajos.
- Estados: recibido, en proceso, esperando repuesto, finalizado y entregado.
- Costos de repuestos y mano de obra, importe acordado, total pagado, saldo y margen estimado.
- Registro de pagos parciales, impresión de ficha de trabajo, búsqueda global, filtros y avisos de próximos servicios por fecha.
- Panel de indicadores y finanzas, exportación CSV de trabajos, respaldo y restauración JSON.
- PWA instalable y usable offline (después de primera carga), sin dependencias externas de ejecución.
- Ejemplos ficticios iniciales, señalizados y eliminables desde la interfaz.

## Persistencia y privacidad

Los datos se guardan **solo en IndexedDB del navegador**; el código estático se publica en Pages pero los registros de clientes **no se envían a GitHub**. La aplicación NO permite compartir información entre dispositivos, no tiene usuarios ni permisos, no cifra los registros, no realiza backups automáticos y no sustituye software contable/fiscal. No se recomienda usar como única fuente para información crítica; descargar respaldos JSON frecuentes.

La demostración aparece solo en el primer inicio cuando no hay datos anteriores. Para utilizar datos reales, elegir «Comenzar desde cero» antes de registrar información. Borrar los datos del navegador elimina los registros locales; exportar JSON permite restaurarlos. **Importar reemplaza** la totalidad de los datos actuales y pide confirmación.

## Desarrollo local

No necesita npm ni compilación:

```bash
python3 -m http.server 8000 -d site
# Abrir http://localhost:8000
```

Para comprobar sintaxis y pruebas:

```bash
node --check site/app.js
node --test tests/*.test.mjs
```

## Publicación

El workflow `.github/workflows/pages.yml` corre en cada push a `main` (o manualmente), valida archivos y pruebas, y publica únicamente `site/` con `actions/upload-pages-artifact` y `actions/deploy-pages`. Debe estar configurado **Settings → Pages → Build and deployment → Source: GitHub Actions** (el workflow intenta habilitar Pages con `enablement: true`).

El manifest y las URLs de recursos son relativas al directorio del sitio, por lo que soporta la ruta base `/AutoTrace/`.

## Estructura

- `site/index.html`: documento HTML y diálogos accesibles.
- `site/app.js`: vistas, formularios, acciones, persistencia y validaciones.
- `site/styles.css`: diseño adaptable a móvil y escritorio.
- `site/sw.js`: caché offline de recursos estáticos, sin interceptación de datos.
- `site/manifest.webmanifest`: instalación PWA.
- `tests/`: comprobaciones automatizadas.

## Próxima etapa posible

Autenticación, sincronización de datos y respaldo server-side, roles, adjuntos/fotografías, recordatorios, turnos, inventario y presupuestos detallados. Cada capacidad necesita un backend con control de acceso apropiado; no usar Pages para alojar información privada compartida.
