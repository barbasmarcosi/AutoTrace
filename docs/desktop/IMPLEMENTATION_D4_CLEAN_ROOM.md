# D4 — Validación independiente de recuperación en datos poblados

Prueba integrada Rust `clean_room_restore_from_both_sqlite_and_excel_preserves_realistic_records`:

1. Base SQLite A con cliente, patente, vehículo, orden, notas y importes parciales con centavos.
2. Genera un respaldo doble verificado.
3. Elimina la conexión al origen, simulando pérdida de la base activa.
4. Restaura **por separado** desde SQLite a base B limpia y desde XLSX a base C limpia.
5. Comprueba igualdad de JSON canónico, clave foránea, fecha, moneda e importes en ambas.
6. Corrompe el Excel emparejado y exige que un backup SQLite con manifiesto deje de considerarse íntegro.

La CI de Windows ejecuta esta prueba de Rust. **No sustituye** instalar NSIS en otra computadora ni probar un dispositivo externo físico; esos gates siguen pendientes.
