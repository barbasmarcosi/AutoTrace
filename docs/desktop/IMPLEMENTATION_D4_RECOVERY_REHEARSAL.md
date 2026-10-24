# D4: prueba de recuperación sin modificar registros

- En Ajustes > **Probar recuperación** se selecciona un manifiesto `.manifest.json` de cualquier respaldo AutoTrace.
- El motor valida nombre/snapshotId, integridad SQLite, llaves foráneas, checksum de ambos archivos y paridad lógica del XLSX.
- Crea **dos bases SQLite nuevas, en memoria**, recupera de manera independiente desde SQLite y desde Excel, valida integridad, y compara el contenido con el original.
- Si el backup ha sido alterado, falta alguno de los archivos o el Excel no reproduce las relaciones/importes, la prueba devuelve ERROR; no toca los registros actuales.
- El informe presenta los conteos recuperados.

**Restricción importante:** para configurar un backup adicional ahora se exige una unidad/volumen distinta de la ubicación de datos primarios. Esto reduce el riesgo de daño por fallo lógico de volumen, pero **no garantiza** disco físicamente independiente, resistencia frente a robo/incendio, cifrado ni recuperación desde otro equipo. El usuario debe probar también con soporte real y mantenerlo seguro.

## Estado
Implementación destinada a tests Rust/Windows y QA manual. El PR sigue en Draft. Faltan pruebas de instalación, disco extraíble real y cifrado antes de sincronizar a nube.
