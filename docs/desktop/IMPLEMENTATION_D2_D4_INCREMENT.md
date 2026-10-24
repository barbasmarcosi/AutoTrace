# Incremento D2–D4: SQLite y Excel de ida/vuelta (validación pendiente)

**Estado:** CODE IMPLEMENTED / CI PENDING. Esta rama no está lista para clientes reales. Los gates de restauración desde una PC limpia siguen abiertos.

## Cambios reales

- Se agregan módulos Rust `excel.rs` y `backup.rs` a la app Tauri.
- Exportación XLSX completa con las hojas `Manifest`, `Clientes`, `Vehiculos`, `Trabajos`, `Configuracion` y columnas del contrato v1.
- Importador estricto: valida firma, versión, manifest, filas, relaciones, fechas, valores, checksum canónico y rechaza hojas no esperadas y fórmulas.
- El selector abre diálogo nativo para elegir el archivo; muestra confirmación con conteos antes de reemplazar.
- El reemplazo usa la transacción y revisión optimista de D1 y exige primero crear una copia del estado anterior.
- `backup_now`: genera instantánea SQLite mediante Online Backup, exporta XLSX desde esa copia, verifica igualdad lógica, integridad y SHA-256 de los archivos, publica `manifest.json` como marcador final.
- Se agenda backup cuando la app está abierta y han pasado al menos cuatro horas; funciona localmente sin servicio de fondo.
- Exportación manual desde diálogo nativo; si existe un archivo destino no se sobrescribe.
- La PWA en Pages y su almacenamiento IndexedDB permanecen sin cambios de comportamiento. El frontend invoca funciones nativas solamente dentro de Tauri.

## Límites conocidos y trabajo restante

- Las copias automáticas están **en el mismo equipo** y no son protección ante fallo total del dispositivo. Aún falta configurar un destino externo automático, retención y cifrado opcional.
- Un Excel de respaldo completo contiene datos internos: no entregarlo a clientes. La exportación XLSX filtrada para el cliente es un hito distinto pendiente; el resumen imprimible ya existe.
- La importación XLSX editada manualmente se rechaza si no conserva la suma de verificación; falta modo guiado de edición con vista previa de diferencias.
- La previsualización de importación muestra cantidades y archivo, no diffs por registro. No hay rollback de instalador ni recuperación demostrada desde otro equipo.
- Por ahora la automatización depende de que la app se abra; no promete respaldo con equipo apagado o app cerrada.
- La biblioteca XLSX se limita a un archivo de entrada de 25 MB; aún falta un límite explícito de expansión ZIP/anti-zip-bomb antes del parser.
- Las copias SQLite tienen coherencia transaccional, pero la política de retención y restauración con archivos externos requiere pruebas operativas independientes.

## Pruebas exigidas

- `cargo test --all-targets` en Windows: pruebas de XML Excel y SQLite con los tres tipos de entidad.
- `node --test tests/*.test.mjs` y SQLite Python.
- Restauración completa en PC limpia, corrupción intencional, fallos de disco, diálogo cancelado, corte durante exportación, datos con ceros iniciales y fórmulas Excel.

**No mergear a main ni ofrecer como release hasta completar pruebas de recuperación y seguridad de importación.**
