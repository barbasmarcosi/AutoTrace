# Primer incremento ejecutado — D0 / D1 (en verificación)

- **Base**: `45ce645` de AutoTrace (main), PR #1 en Draft. No se modifica la demo de Pages.
- **Entregado en código**: shell Tauri v2, base SQLite versionada, relaciones/constraints/indexes, validación Rust de entidades, guardado transaccional con revisión optimista, frontend dual IndexedDB/SQLite, nuevo entorno de CI.
- **Migración de datos**: el usuario exporta JSON desde la PWA y lo importa por la UI actual dentro del desktop. El backend valida nuevamente; la importación es un reemplazo transaccional, no un merge.
- **Limitación deliberada**: por ahora se reescribe el conjunto de filas en una sola transacción cada vez que se guarda; sencillo para un único usuario, pendiente optimizar operaciones incrementales y validar rendimiento con grandes volúmenes.
- **Sin demo en desktop**: primer arranque retorna arrays vacíos; IndexedDB original sigue igual cuando se accede por web.
- **Privacidad**: SQLite vive en directorio local de la aplicación, sin servidor. No hay backup automático aquí todavía; antes del piloto real es obligatorio completar D3/D4/D5 y restauración de copia externa.
- **No confundir test SQLite de Python** con compilación de Rust: la CI incluye ambos; instalar una build Windows y verificar con WebView2 es un gate pendiente.

## Gates del incremento

1. `node --check site/app.js` y `node --test tests/*.test.mjs` (entorno JS).
2. `python3 -m unittest discover -s tests -p 'test_desktop_schema.py' -v` (SQLite real).
3. `cargo test --all-targets` con runner Windows (GitHub Actions); inspeccionar resultado antes de cerrar D1.
4. Prueba manual en Windows: alta de cliente, vehículo, trabajo, pago parcial, cierre, nueva apertura. Esta prueba no se afirma realizada.
5. D2: comparar import JSON y valores después de exportar/reiniciar; no borrar origen nunca.

**Estado:** experimental, mantener PR Draft. La app desktop aún no debe gestionar la única copia de información real.
