# AutoTrace — Contrato de calidad 10/10

**Corte inicial:** 2026-10-24. **Estado:** cumplimiento parcial; no declarar listo para producción.  
**Candidato de integración actual:** [`integrate/desktop-quality-rc-20261024`](https://github.com/barbasmarcosi/AutoTrace/pull/16) — PR #16 (**Draft**), basado en `main`. Los PR #1–#15 siguen como referencia y no se fusionaron en producción.  
**Objetivo:** una persona maneja un taller en **una sola PC Windows**, sin servidor obligatorio, y ante pérdida total del equipo recupera el **100 % de los datos** en una instalación limpia usando copia externa y kit seguro.

## 0. Evidencia CI y límites de la validación (24/10/2026)

El commit `7da0d2ba2c01fc752a50ed827e3c7d35619c4cd7` del PR #16 tiene los tres workflows **PASS**:

- [Installer NSIS, build e install/uninstall en runner Windows](https://github.com/barbasmarcosi/AutoTrace/actions/runs/40066294073)
- [Desktop foundation (Rust y dominio)](https://github.com/barbasmarcosi/AutoTrace/actions/runs/40066294104)
- [Chromium/browser smoke](https://github.com/barbasmarcosi/AutoTrace/actions/runs/40066294070)

**Alcance:** valida ese commit y ese runner efímero; NO valida USB físico, recuperación en otra PC, Google OAuth real ni UX WebView2 con varios DPI.

**Nueva tanda de G0/G4:** los cambios posteriores endurecen el disparo de CI al modificar cualquier archivo Desktop, test o sitio, y añaden regresiones de archivos `.atbackup.age` truncados al recorrido de ensayo/preview. Su condición inicial es **pendiente hasta verificar CI del HEAD actualizado**; no heredar el PASS del SHA anterior.

**Decisión de publicación:** conservar PR #16 en Draft, sin merge a `main`, sin release productivo. Una edición offline sin Google solo puede publicarse si se aprueban expresamente G0–G4 y G6–G8, y G5 se comunica como función excluida.

## 1. Definición verificable de 10/10

**10/10 no es una calificación subjetiva ni una CI verde:** significa pasar **todos** los gates críticos G0–G8 con evidencia reproducible y aprobación funcional; no es necesario incluir funcionalidades que exceden el alcance de una PC. Mantener **Draft** si alguno no pasó.

| Gate | Evidencia de aceptación exigida | Estado al crear el documento |
|---|---|---|
| **G0 — Compila el SHA candidato** | Node, migraciones SQLite, Rust Windows, Chromium, NSIS, instalación/desinstalación sobre **mismo commit**, sin fallos ni cancelaciones | **PASS en `7da0d2ba` (PR #16)**: tres workflows verdes. Revalidar íntegramente cualquier SHA posterior. |
| **G1 — Datos íntegros** | CRUD, relaciones, revisiones concurrentes, 0 huérfanos, validaciones y 8 hojas Excel v3, redondeos y cobros equivalentes tras round-trip | Automatizados parcialmente; requieren mayor cobertura sobre volumen |
| **G2 — Copia externa demostrable** | Backup SQLite+XLSX consistente, manifiesto SHA-256, archivo age; destino fuera del volumen de datos; ensayo con USB físico y sin red | Doble reconstrucción automatizada; no hay ensayo físico |
| **G3 — Recuperación ante desastre** | PC limpia sin archivos del origen; importar **solo** `.atbackup.age` y frase conservada por separado; comparar IDs, clientes, autos, órdenes, detalles, cobros, cuentas y config | **NO PROBADO** en otra PC |
| **G4 — Falla segura** | Cortar USB durante escritura, sin espacio, ZIP/XLSX truncado, checksum incorrecto, frase incorrecta, base más nueva, rollback y simultaneidad | Parte automatizada, inyección de fallos insuficiente |
| **G5 — Google Drive real** | OAuth con Client ID público integrado, permiso `drive.file`, refresh, 308, corte de red, cuotas, no duplicar, confirmar MD5/tamaño, restaurar en otra PC | **BLOQUEADO**: falta identidad Google Cloud y cuenta de test |
| **G6 — Seguridad y privacidad** | Tokens solo en vault, age antes de cloud, nada privado en logs/Actions, permisos mínimos, versiones/dependencias controladas, auditoría de vulnerabilidades | Parcial; auditar cadena de suministro y amenazas |
| **G7 — Experiencia humana** | Operador no técnico completa alta, presupuesto, cobro, backup, aviso y recuperación; pruebas WebView2 100/125/150 % DPI, teclado y dispositivos | Parcial; ensayo manual Windows pendiente |
| **G8 — Release mantenible** | Integración revisable sin conflictos de Pages, upgrade y rollback en Windows limpio, checksum/versión, firma o aviso claro | PR #16 consolida cambios en rama Draft; faltan upgrade, rollback, firma/advertencia y release real |

**Aprobación:** solo liberar cuando G0–G8 tengan evidencia y no quede ningún P0/P1 de pérdida de datos abierto. Si Google Cloud se retrasa, puede existir un release **offline** explícitamente sin Google, siempre que G0–G4 y G6–G8 estén aprobados y G5 figure como excluido del alcance publicado.

## 2. Hallazgos del cierre de calidad y correcciones aplicadas

| Hallazgo | Acción | Evidencia |
|---|---|---|
| Rust del PR #12 fallaba con `E0428/E0252` por dos `#[tauri::command]` en `generate_recovery_phrase` | Quitado atributo duplicado | Compilación Windows del PR #13 pendiente |
| Hash de respaldos SQLite/Excel cargaba el archivo completo en RAM | SHA-256 por streaming en bloques de 128 KiB | Código `backup.rs`, CI Rust pendiente |
| Una copia podía publicar manifiesto sin sincronizar contenido al SO | `sync_all` de SQLite, XLSX y JSON antes de renombrar el manifiesto | No sustituye ensayo real de pérdida de energía |
| Un usuario podía reemplazar accidentalmente la frase guardada | Fail-closed cuando la nueva frase difiere de la almacenada en Windows o de la sesión; revisar archivos cifrados históricos (extremos) de carpeta sincronizada | Rotación oficial de clave sigue pendiente |
| Exportación portátil permitía guardar en el mismo volumen | Bloqueo de destino en la misma unidad/volumen; mantiene checksum SHA-256 de la copia cifrada exportada | Otra letra/unidad no prueba disco físico distinto |
| Exportación manual generaba nombres locales fuera de política de retención | IDs normales timestamp+UUID para entrar en retención conservadora | CI y prueba de ocupación pendientes |
| Pruebas UI nativas eran mayormente estáticas | Nuevo smoke Chromium con `__TAURI__` sintético: kit, frase duplicada, exportación, Google desactivado | No implica acceso real al vault, Drive ni USB |
| Browser CI omitía ramas `fix/**` | Se añadió activación en branches `fix/**` y `pull_request` | Resultado browser del SHA final pendiente |

## 3. Prioridades posteriores (sin expansión de alcance)

**P0. Sostener G0:** las tres CI del commit `7da0d2ba` son verdes; los cambios posteriores requieren repetir los tres checks sobre el nuevo HEAD. No publicar instaladores de SHA anteriores como versión actual.

**P0. Cerrar G2/G3/G4:** preparar ensayo físico Windows con fixture sintético. Comprobar captura, exportación age al USB, retirada de la unidad, nueva instalación y prueba de recuperación, con evidencias anonimizadas. Desconectar el USB durante exportación; debe abortar sin pérdida de base. Windows moderno puede cachear escrituras: sincronización de archivos no garantiza persistencia si hardware no respeta flush.

**P0. Cerrar G8:** PR #16 resuelve la dificultad de revisión de PR apilados incorporándolos en una rama derivada de `main`, preservando `.github/workflows/pages.yml`. Aún falta validar upgrade, compatibilidad de bases anteriores, rollback y checklist de distribución. No fusionar antes de evidencia.

**P1. Dinero y auditoría:** asegurar importes enteros en centavos/decimal exacto extremo a extremo y transacciones del historial de cobros que admitan anulación y auditoría sin borrados.

**P1. Rendimiento:** pruebas con 1k/10k/50k trabajos; analizar sustitución de `store::write` que actualmente elimina/reinserta la totalidad de las filas dentro de cada transacción. Si se cambia, conservar revisión optimista y restaurabilidad.

**P1. Suministro reproducible:** generar y versionar `Cargo.lock` verificable, fijar versión CLI Tauri, revisar dependencias y permisos, procesar hallazgos de `cargo audit` y herramientas equivalentes. No afirmar reproducibilidad hasta contar con lockfile y artefactos verificables.

**P1. Respaldo cifrado y rotación:** definir identidad/versionado de clave y migración del histórico sin cambiar silenciosamente la contraseña; limpiar restos de descifrado temporal al iniciar después de interrupción.

**P1. Probar reportes Excel** contra documentos reales del taller sin filtrar información sensible a GitHub. Conservar solo fixtures sintéticos. Respetar que estos documentos NO son comprobantes fiscales.

**Fuera de alcance de este cierre:** agenda, inventario, adjuntos, multiusuario, sincronización activa entre múltiples PCs. Son mejoras posteriores que no deben retrasar G0–G8.

## 4. Evidencia y manual del ensayo físico

- **Dispositivo A:** en Windows, registrar fixture ficticio con 2 clientes, 2 autos, 3 órdenes, detalle, saldos, 3 cobros y 2 cuentas; generar kit y guardar frase en papel.
- **Dispositivo A:** exportar a disco USB secundario; registrar SHA-256 del `.atbackup.age`, fecha, versión Desktop y tamaño; hacer un ensayo no destructivo (SQLite y XLSX). No incluir PII real.
- **Dispositivo B:** instalación limpia sin acceso a perfil, DB ni claves de A; sin red, conectar USB y ejecutar «Probar copia cifrada portátil»; verificar cantidades y todos los campos, luego importar con confirmación.
- **Dispositivo B:** comprobar que cambiar una letra de la frase o un byte cifrado rechaza la recuperación sin alterar el estado. Verificar posibilidad de rollback si B ya contenía datos.
- **Proveedor:** repetir con Google cuando esté disponible, descargando archivos cifrados desde cuenta real y comparando SHA-256 y datos recuperados.
- **Registro:** SHA de código, hash archivo, Windows build, resultado, fecha y firma del evaluador. Si falla cualquier gate, volver a Draft.

**Prohibiciones:** secretos reales en CI, afirmar recuperación comprobada por un test de DOM, subir SQLite/Excel sin cifrar, usar una letra de unidad como prueba de dispositivo físico independiente, o mezclar el Client ID de desarrollo con el de producción sin migración.
