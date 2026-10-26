# AutoTrace — Contrato de aceptación 10/10 por responsable

**Fecha: 2026-10-24. ESTADO: NO-GO.** Este contrato establece condiciones de aceptación: no demuestra por sí solo que se cumplieron.

## Alcance

Una PC Windows para un taller, operación local offline sin servidor, clientes, vehículos, órdenes, presupuestos, cobros, cuentas, Excel y backups cifrados independientes. La sincronización directa con Google Drive pertenece a la edición completa; una edición offline podrá entregarse por separado y etiquetarse sin Drive. Multiusuario, agenda, inventario y facturación fiscal quedan fuera de esta versión.

**Aprobación 10/10 completa: todos E0–E8 y U0–U7 en PASS, basados en el mismo commit de release, sin P0/P1 de pérdida de datos ni seguridad pendientes.** No convertir «no probado» en «aprobado». Cada nueva corrección invalida las pruebas de SHA anteriores que deban repetirse.

## Ingeniería / ChatGPT: E0–E8

| Gate | Condición verificable a cargo de ingeniería | Evidencia para PASS |
|---|---|---|
| E0 Compilación | Node, migraciones, Rust tests en Windows, Chromium, NSIS, instalación/desinstalación **en el mismo SHA**. | Enlaces a cuatro workflows, checksum del instalador y SHA coincidente. |
| E1 Datos | CRUD, relaciones y cobros sin huérfanos; importes enteros en centavos, saldos, historial, errores concurrentes, migraciones, ocho hojas Excel v4 con lectura retrocompatible v1–v3 y trazabilidad de anulación de cobros. | Tests sintéticos automatizados, escenarios y aserciones en SQL/JS/Rust. |
| E2 Respaldo | SQLite y Excel consistentes, manifiesto y SHA-256, cifrado antes de red/USB; no sobrescribir copia buena, publicar parciales ni afirmar sincronización prematura. | Round-trip y pruebas de integridad, colisiones, cortes simulados y staging. |
| E3 Fallos seguros | Clave errónea, corrupción, truncado, disco lleno simulado, conflictos, rollback y versión de DB futura no destruyen estado previo. | Tests con estado antes/después del fallo, no mocks permisivos. |
| E4 Google sintético | OAuth PKCE S256/state/loopback, scope mínimo drive.file, refresh en vault; HTTP 308, errores, reintentos, confirmación de tamaño/MD5, sin duplicados. | Tests sin cuentas reales, secretos ni tráfico Google de clientes. |
| E5 Seguridad | Cargo.lock, RustSec sin vulnerabilidades relevantes; warnings evaluados; mínimo permiso de Tauri/CI, CSP, sin tokens o SQLite/Excel privados en acciones. | Auditoría real del lock y políticas de seguridad que fallen cerrado. |
| E6 Rendimiento | Escala 1k/10k/50k órdenes, integridad después de operaciones y métricas medidas con entorno declarado, objetivos fijados antes de aprobar. | Benchmark reproducible, métricas de escritura/lectura/edición y memoria. |
| E7 UX automatizable | Flujos básicos, teclado, modales, scroll, fallos y offline bajo pruebas de navegador. | Suite Chromium; **nunca** representar este test como prueba física WebView2. |
| E8 Distribución | NSIS, checksum, smoke de reinstalación sin pérdida, upgrade histórico automatizable, sin publicar Pages accidentalmente, pasos de rollback/firma. | Artefactos y CI del SHA final, PR revisable y política NO-GO. |

**Responsabilidad técnica:** implementar E0–E8, corregir fallas reproducibles, dejar artefactos, pruebas, métricas y resultados vinculados al SHA. Si aún faltan mediciones o una prueba, estado PENDING.

## Propietario/usuario: U0–U7

| Gate | Prueba en entorno real | Resultado para PASS |
|---|---|---|
| U0 Instalar | Ejecutar instalador con checksum correcto en PC Windows A; reiniciar, abrir, verificar SmartScreen/antivirus. | Instalación y uso básicos reales. |
| U1 Operar | Cargar clientes, autos, trabajos, presupuestos y pagos **ficticios**; cerrar/reabrir e importar/exportar Excel. | IDs, saldos y datos coinciden; operador no técnico puede continuar. |
| U2 USB externo | Crear respaldo cifrado en unidad USB de **otro disco físico**, guardar frase en papel distinto, verificar SHA-256. | Respaldo legible fuera de A, sin copias claras en USB. |
| U3 Recuperación | Restaurar desde USB + frase en PC Windows B limpia, sin Internet, cuenta, perfil ni DB de A. | Coinciden IDs, líneas, clientes, autos, cobros, saldos, cuentas y Excel. |
| U4 Fallos físicos | Desconectar USB en escritura, disco lleno, clave incorrecta, archivo corrupto y pérdida total de A. | DB vigente y último backup sano sobreviven; recuperación real funciona. |
| U5 Google real | Crear Client ID Desktop propio de AutoTrace, autorizar cuenta de prueba, subir, cortar/reintentar, revocar/refrescar, bajar y restaurar en B. | Recuperación remota de extremo a extremo, no solo login. |
| U6 WebView2 | Con operador real en Windows probar teclado, foco, modal, scroll y escalas 100/125/150 % DPI. | Sin bloqueo funcional ni problemas de accesibilidad críticos. |
| U7 Upgrade | Instalar versión anterior con datos ficticios, actualizar y revertir según procedimiento documentado. | No se pierden datos ni backups; firma o falta de firma declarada. |

**Evidencias U**: fecha, commit SHA, versión de Windows, checksum del EXE/USB, resultado, notas y capturas anonimizadas. **Jamás** publicar contraseñas, tokens o datos reales de taller. Instrucciones detalladas en [PHYSICAL_RELEASE_ACCEPTANCE.md](./PHYSICAL_RELEASE_ACCEPTANCE.md).

## Proceso y decisión

1. Primero cerrar E0–E8 en PRs, con evidencia repetible del HEAD final.
2. Solo entonces entregar un paquete concreto de aceptación (EXE + SHA + fixtures ficticios + runbook) para U0–U7. El paquete del PR #36 incluye manifiesto y verificador sin firmas; sus preflights no implican U aprobados.
3. Cada FAIL de U vuelve a ingeniería; corregir y repetir las pruebas afectadas en un nuevo SHA.
4. Si cualquier P0/P1 de seguridad o pérdida de datos sigue abierto: NO-GO, PR de release en Draft y main sin publicar.
5. Drive de ChatGPT **no es** OAuth de AutoTrace. U5 exige configuración real de Google Cloud (Client ID público de tipo Desktop) bajo control del propietario.

## Registro inicial

- [PR #36 candidato integrado](https://github.com/barbasmarcosi/AutoTrace/pull/36): Draft, NO-GO; exigir evidencia E0–E8 del SHA final.
- [PR #52 aceptación física](https://github.com/barbasmarcosi/AutoTrace/pull/52): paquete de U0–U7 incorporado a la rama candidata #36 con contenido idéntico.
- [PR #19 dependencias](https://github.com/barbasmarcosi/AutoTrace/pull/19): Draft, mitigación de ZIP.
- [PR #20 cifrado](https://github.com/barbasmarcosi/AutoTrace/pull/20): Draft, ensayos de compatibilidad age pendientes de CI final.
- [Issue #17 aceptación física/OAuth](https://github.com/barbasmarcosi/AutoTrace/issues/17).
- [Issue #18 auditoría RustSec](https://github.com/barbasmarcosi/AutoTrace/issues/18).

Este documento **define** el 10/10; no adjudica PASS a ninguna validación sin evidencia.
