# AutoTrace Desktop — índice de planificación

**Estado:** propuesta técnica, sin implementación aún. **Objetivo:** una instalación en una sola computadora y un solo usuario, sin servidor ni suscripción obligatoria; mantener la PWA de GitHub Pages como demostración.

- [Plan de ejecución, arquitectura y fases](PLAN.md)
- [Contrato del Excel reversible e importable v1](EXCEL_FORMAT_V1.md)
- [Política de copias automáticas y recuperación](BACKUP_AND_RECOVERY.md)
- [Matriz de aceptación y pruebas](ACCEPTANCE_MATRIX.md)

## Decisiones de producto

1. **Persistencia primaria:** SQLite en la computadora del taller, no el navegador ni una base de datos remota.
2. **Aplicación:** Tauri v2, reutilizando la UI HTML/CSS/JS actual mediante un adaptador de datos; Windows 10/11 de 64 bits como primer objetivo de entrega (validar el SO del cliente antes de empaquetar). No se exige internet para operar.
3. **Respaldo automático doble:** instantánea SQLite **más** Excel XLSX completo e importable, generados del **mismo snapshot**. SQLite es el artefacto de recuperación más fiel; Excel es una segunda vía interoperable y legible. No copiar el archivo SQLite en uso con un simple copy/paste.
4. **Exportación manual:** «Exportar todo a Excel» (privado, reimportable), «Excel del cliente» (datos filtrados y sin información interna) y «Restaurar/Importar Excel» con previsualización, validación y confirmación.
5. **Seguridad:** copias locales con permisos acotados; copias fuera del equipo cifradas. Un Excel XLSX común **no está cifrado**. Separar respaldo confidencial de informe destinado a un cliente.
6. **No afectar producción:** mantener `main` y la PWA desplegada intactos durante desarrollo; no sustituir IndexedDB ni borrar datos hasta validar migración y recuperación real.

## Fuera de alcance inicial

Multiusuario, SaaS, backend HTTP, suscripciones, roles avanzados, facturación electrónica, sincronización entre computadoras, inventario, adjuntos binarios y pagos individualizados retroactivos. Los importes ya existentes (`charged` y `paid`) se conservan tal cual; no se inventa historial de cobros.

## Referencias técnicas verificadas

- Tauri SQL/migraciones: https://v2.tauri.app/plugin/sql/
- Instalador Windows NSIS/MSI: https://v2.tauri.app/distribute/windows-installer/
- Backup API de SQLite: https://www.sqlite.org/backup.html
- Tauri filesystem y seguridad: https://v2.tauri.app/plugin/file-system/
- El esquema de origen continúa definido en `site/model.mjs`.

**Estado de este PR:** documentación y criterios de aceptación; no significa que ya existan instalador, SQLite, XLSX automático, restauración ni pruebas ejecutadas.

## Avance de implementación: D0/D1
Se incorporó la primera implementación Tauri/SQLite y el puente de persistencia dentro de la rama de este PR (ver [IMPLEMENTATION_D0_D1.md](IMPLEMENTATION_D0_D1.md)). **No es un instalador entregable todavía**: la CI Windows debe compilar correctamente y faltan backup automático, Excel importable, restore y ensayo de recuperación externa. El PR continúa Draft.
