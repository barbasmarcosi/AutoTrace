# AutoTrace — protocolo de aceptación física y release (PR #16)

**Estado inicial:** PENDIENTE. Este documento NO certifica que se hayan realizado pruebas en hardware. Usar instalador **del SHA final**, no un preview anterior. Ningún dato real ni contraseña de recuperación se sube a GitHub Actions.

## Identificación de evidencia

| Campo | Completar |
|---|---|
| Fecha UTC / evaluador | PENDIENTE |
| Commit SHA de PR #16 | PENDIENTE |
| Workflow y SHA-256 del instalador NSIS | PENDIENTE |
| Windows A (versión y DPI) | PENDIENTE |
| Windows B limpio (versión y DPI) | PENDIENTE |
| Disco USB independiente (número/bus, hash) | PENDIENTE |
| Estado final y referencia a evidencia sanitizada | PENDIENTE |

## Ensayos obligatorios (marcar PASS únicamente con observación directa)

1. **G0 / distribución:** descargar EXE+SHA del mismo workflow del HEAD. Comprobar SHA-256 antes de instalar. No omitir advertencia de binario sin firma. Instalar y abrir en Windows real; reiniciar, reinstalar, actualizar desde una versión anterior compatible y desinstalar sin borrar la base. La reinstalación sobre idéntico NSIS en CI no demuestra una actualización histórica.
2. **G1 / información:** crear exclusivamente datos ficticios: 5 clientes, 7 vehículos, 12 trabajos en distintos estados, renglones y pagos parciales/completos, dos cuentas receptoras, patente con cero y Unicode. Comprobar IDs, sumas exactas en centavos, saldos y exportación/importación de Excel con ocho hojas. Cerrar y reabrir, cotejar.
3. **G2 / respaldo independiente:** preparar USB físico diferente al disco principal. Exportar un `.atbackup.age` y conservar la frase de recuperación **fuera** de A y del USB. Ejecutar `scripts/collect-physical-recovery-evidence.ps1`, que genera un JSON sin nombres/rutas/personales; registrar SHA-256. Retirar físicamente USB, comprobarlo en otra PC. Un volumen lógico diferente no basta.
4. **G3 / desastre:** B debe ser una instalación Windows limpia **sin** perfil, credenciales ni base de A, desconectada de Internet. Instalar AutoTrace desde NSIS y conectar SOLO el USB con la copia cifrada. Ejecutar «Probar copia cifrada portátil», luego recuperar con la frase guardada en papel y comparar ID y campos de todos los registros, detalles y cobros. Cotejar estado y cuentas y exportar Excel nuevo. Verificar que A estuvo inaccesible todo el tiempo.
5. **G4 / fallas:** repetir con frase incorrecta, byte alterado, copia truncada y copia anterior. La app debe rechazar SIN tocar la base B. Durante copia a USB, retirar USB y probar lleno/sin permiso; nunca debe indicar éxito ni eliminar la última copia buena. Si B tenía datos, exigir copia previa antes de reemplazar y demostrar rollback. Probar DB de versión futura: rechazo sin migración destructiva.
6. **G5 / Google:** cuando exista una **identidad OAuth Desktop administrada por AutoTrace**, instalar build con Client ID público. Autorizar cuenta de prueba en navegador externo con PKCE, scope `drive.file`; reiniciar, comprobar refresh/vault, cortar Internet a mitad de carga, reanudar y evitar duplicaciones. Corroborar tamaño+MD5 de archivo cifrado en Drive y restaurarlo en B sin archivos locales de A. Probar revocación OAuth y cuota agotada. **Sin identidad ni cuenta real: G5 permanece BLOCKED, nunca PASS.**
7. **G6 / privacidad:** revisar antivirus/SmartScreen, permisos, registros y artefactos de CI; no exponer tokens/clave, SQLite/Excel sin cifrar, rutas privadas ni backups del taller. Adjuntar resultado de RustSec y evidencia de revisión de dependencias.
8. **G7 / UX:** operador no técnico completa alta, presupuesto, cobro, backup y restauración sin instrucciones de desarrollo. Probar Windows WebView2 al 100, 125 y 150 % DPI, teclado, foco, modal, scroll, cierre y arranque offline. Registrar problemas P0/P1 y corregirlos.
9. **G8 / release:** comparar migración desde versión previa con backup anterior, validar rollback y reinstalación limpia, checksum y procedencia del EXE. Elegir firma de código o comunicar explícitamente «instalador de prueba sin firma». Solo publicar con evidencias, CI verde sobre el SHA definitivo y aprobación.

## Evidencias que nunca equivalen a PASS físico

- GitHub Actions en un runner efímero, pruebas Chromium con puente Tauri sintético, mocks HTTP o archivos en dos carpetas del mismo disco.
- Que el instalador arranque, o que el backup incluya `SHA-256`, sin comprobar una restauración en B.
- La existencia de un refresh token, o el éxito de una solicitud HTTP, sin validación remota y recuperación.
- El informe JSON `MEDIA_PREFLIGHT_ONLY`: solo prueba soporte físico distinto y hash del archivo, no su contenido ni la restauración.

## Decisión

Si cualquier P0/P1 de pérdida de datos o recuperación sigue abierto: **NO-GO**. Mantener PR #16 en Draft y main sin alteraciones.

Puede plantearse posteriormente una edición **offline** sin Drive, pero solo con G0–G4 y G6–G8 demostrados y sin representar G5 como completado. El producto completo 10/10 requiere G0–G8.
