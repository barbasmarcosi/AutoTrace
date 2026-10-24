# AutoTrace Desktop — implementación de escritorio (preview)

AutoTrace usa Tauri v2 y SQLite local, sin servidor y sin cuotas mensuales obligatorias. Reutiliza la interfaz de `../site/`. La misma UI en navegador sigue utilizando IndexedDB y GitHub Pages; en Windows Desktop utiliza comandos nativos validados y SQLite.

## Características implementadas en este PR

- Modelo SQLite con restricciones, migración v1, escritura transaccional, revisión CAS e importación desde JSON de la PWA.
- Exportación e importación de Excel completo con ID estables y validación, más informe Excel por cliente sin datos privados de otros clientes ni costos internos.
- Respaldos automáticos locales SQLite + XLSX + manifiesto SHA-256 del mismo snapshot; opción de replicarlos a una **unidad/volumen distinto**.
- Retención prudente, indicadores de copia verificada, recuperación con respaldo previo y opción de **probar recuperación** desde ambos formatos en bases temporales.
- Copias locales y a volumen secundario **no cifradas**: no sincronizarlas directamente a una nube.
- **Google Drive para escritorio (D6, en Draft):** copia opcional con cifrado age de SQLite + Excel + manifiesto dentro de un solo `.atbackup.age` en una carpeta sincronizada. Frase de recuperación solo en memoria durante la sesión; recuperación validada desde el archivo cifrado. No se usan API keys ni permisos de Google, ni se confirma automáticamente la sincronización remota. Ver [guía D6](../docs/desktop/D6_GOOGLE_DRIVE_ENCRYPTED.md).
- Instalador **NSIS unsigned** generado con workflow Windows, con CI Rust/SQLite/Node y prueba de instalación/desinstalación silenciosa en runner descartable (solo tras su primera ejecución verde).

## Google Drive API directa (PR #10 · preview)

En **Ajustes → Google Drive · Conexión directa** se puede configurar un OAuth Client ID de tipo Desktop, autorizar una cuenta Google mediante PKCE y guardar el token de renovación en **Windows Credential Manager**. Tras desbloquear la frase `age`, las copias SQLite + Excel + manifiesto se cifran y se suben por la **Google Drive API v3** a una carpeta propia visible (`AutoTrace Backups`), con confirmación remota de tamaño y MD5. Una cola cifrada retiene copias cuando falla la red; se reintenta mientras la aplicación está abierta. Se pueden listar, probar sin destruir datos y restaurar copias remotas.

**Siguiente etapa de producto:** [D8 — Google Drive en un clic](../docs/desktop/D8_GOOGLE_DRIVE_ONE_CLICK_PLAN.md). El Client ID visible en Ajustes es una solución provisoria de desarrollo, no un requisito para el cliente final. D8 también resuelve el desbloqueo automático del cifrado y las pruebas de error sin Google Cloud.

La configuración OAuth externa, prueba de subida y recuperación desde otra PC **NO** están verificadas todavía. La PWA Pages y el instalador anterior a este PR no incluyen estos comandos nativos. Consultar [guía de activación D7](../docs/desktop/D7_GOOGLE_DRIVE_API_DIRECT.md).

## Instalación y pruebas

Requiere Windows 10/11 x64, WebView2 y runtime de Tauri. Para desarrollo: Rust estable, Tauri CLI v2, `cd desktop/src-tauri && cargo tauri dev`.
Para tests: `cargo test --manifest-path desktop/src-tauri/Cargo.toml --all-targets`. Para distribución: el workflow `.github/workflows/desktop-installer.yml` genera un instalador NSIS sin firma; su artefacto se retiene 7 días.

**No es un release productivo.** Falta la prueba en una computadora Windows real con instalación y actualización de la aplicación, recuperación desde un soporte físico distinto, cifrado antes de la nube y aceptación del usuario final. Las pruebas del runner NO demuestran recuperación ante robo/falla física de la PC.

## Migración y respaldo

En la PWA: Ajustes > Exportar JSON. En AutoTrace Desktop: Ajustes > Importar JSON (previa confirmación). Conservá el JSON original.
Los datos del cliente no se cargan en CI, el instalador no contiene datos reales y el sitio Pages no se modifica desde este PR.
