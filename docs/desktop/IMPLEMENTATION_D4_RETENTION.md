# D4 — Retención prudente (2026-10-22)

Se implementó una política de copias verificadas, no una limpieza ciega por extensión:

- Mantener 12 instantáneas recientes, una muestra de 14 días, 8 semanas y 12 meses.
- Solo se consideran candidatos los tripletes con nombre generado por AutoTrace, manifest válido y archivos SQLite/Excel verificados.
- Ante hash roto, nombre extraño, manifiesto incompleto o un archivo ajeno: **no borrar**.
- Jamás eliminar la última copia verificada. La limpieza sucede después de crear/verificar la copia nueva.
- Borrar el manifest primero, de modo que una interrupción no deje archivos incompletos anunciados como respaldos válidos.
- Si falla una limpieza, se conserva el respaldo nuevo y se informa en logs sin revelar información privada.

Pendiente: probar con calendarios reales y verificar límites de espacio/volumen en instalación Windows. El disco externo y la nube no están necesariamente cifrados.
