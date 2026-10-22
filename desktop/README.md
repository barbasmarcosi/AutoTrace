# AutoTrace Desktop — primer incremento (D0/D1)

La interfaz **se reutiliza directamente desde `../site/`**. En navegador, AutoTrace sigue utilizando IndexedDB. Cuando Tauri la abre, usa los comandos nativos `load_state` y `save_state` para persistir en SQLite en el directorio de datos privado de la aplicación. No hay servidor.

## Estado de esta entrega

Implementado en código: shell Tauri v2, migración SQLite v1, relaciones cliente → vehículo → trabajo, restricciones de patente y cobros, guardado por transacción con revision/compare-and-swap, carga con validación y puente de datos de la PWA. Esta **no es aún una versión apta para producción**: falta prueba real de compilación/instalación Windows, respaldo automático SQLite + XLSX, importación Excel, recuperación de equipos y endurecimiento operativo. No usar datos críticos hasta aprobar los gates D2–D5.

## Ejecutar en desarrollo

Requiere Rust estable reciente, dependencias de Tauri v2, WebView2 en Windows y CLI de Tauri:

```bash
cargo install tauri-cli --version '^2' --locked
cd desktop/src-tauri
cargo tauri dev
```

El `frontendDist` apunta a `../../site`; no hay `npm build` ni copia duplicada de HTML. Para pruebas de Rust: `cd desktop/src-tauri && cargo test`. En Windows, el futuro instalador NSIS podrá generarse con `cargo tauri build` tras pasar la CI desktop.

## Reglas de seguridad

- La única escritura permitida desde JavaScript es `save_state` con contenido canónico, tipos controlados y revisión esperada.
- `save_state` valida todos los registros, sus importes, patentes, IDs y fechas, y hace el reemplazo completo en **una transacción**. Se aborta si otra ventana modificó la revisión mientras tanto. Para un usuario es suficiente en esta primera fase; una futura etapa podrá ejecutar cambios por registro para mejorar escalabilidad.
- Los datos nunca viajan a GitHub ni a un endpoint HTTP. No se expone SQL arbitrario a la WebView.
- El primer inicio desktop arranca **sin datos de demostración**. Para migrar desde la PWA hay que exportar su JSON y elegir «Importar JSON» en AutoTrace Desktop. El archivo de origen permanece intacto.
- Sin copia externa, SQLite local **no es suficiente** frente a pérdida/robo de la computadora. Ver `../docs/desktop/BACKUP_AND_RECOVERY.md`.

## Gate verificable

`python3 -m unittest discover -s tests -p 'test_desktop_schema.py' -v` desde la raíz del repositorio confirma las constraints SQLite usando el engine de Python. Esto **no sustituye** la compilación Rust/Tauri en Windows ni un test de recuperación de backups.

## Avance: exportación Excel y respaldo local en rama experimental

Se agregaron comandos nativos `backup_now`, `backup_status`, `export_excel_dialog` y `choose_excel_import`.
El respaldo local produce SQLite + Excel desde el mismo snapshot, con SHA-256 y verificación de integridad. En Ajustes se puede exportar/importar XLSX; toda restauración exige confirmación y copia local previa. Una tarea en primer arranque y cada 4 horas **mientras está abierta la aplicación** comprueba si corresponde generar la copia.

**No confundir con protección ante pérdida de computadora:** aún falta configurar una copia fuera del disco y una restauración probada en otra instalación. El Excel completo incluye datos privados; no es un informe para entregar a clientes. Véase `docs/desktop/IMPLEMENTATION_D2_D4_INCREMENT.md`.
