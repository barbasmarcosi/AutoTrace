# Construcción de instalador Windows NSIS (preview)

El workflow `.github/workflows/desktop-installer.yml` puede ejecutarse manualmente desde Actions; también verifica cambios al propio workflow o a la configuración de Tauri en PR. No crea releases públicas, no fusiona a `main` y no envía datos reales.

- Usa Windows y Tauri CLI para producir `.exe` instalador NSIS, en modo `currentUser` (sin administrador).
- Se ejecutan tests Node, SQLite y Rust antes de empaquetar.
- Se publica solo un artefacto temporal de CI de **7 días**, etiquetado **UNSIGNED**. Puede activar advertencias SmartScreen.
- Se informa SHA-256 en los logs de compilación.
- No se habilita auto-update ni instaladores automáticos en equipos del cliente.
- Generar un instalador no valida todavía instalación, actualización, desinstalación ni recuperación real de respaldos.

Para activar: ir a GitHub Actions > Desktop Windows NSIS installer (preview) > Run workflow y seleccionar rama `plan/desktop-sqlite-excel-backups-20261021`.

**Gate pendiente**: ejecutar el instalador en un Windows limpio, crear registros, exportar copia y restaurarla en otra instalación desconectada antes de entregar.
