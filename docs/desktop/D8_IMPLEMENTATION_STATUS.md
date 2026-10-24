# AutoTrace D8 — Acta de implementación y cierre parcial

**Corte:** 2026-10-24 · Rama `feature/d8-oneclick-offline-hardening-20261024` · **PR #11 (Draft)** sobre PR #10. El sitio estable y la base de los talleres no se modificaron.

Documento operativo complementario al [Plan D8](D8_GOOGLE_DRIVE_ONE_CLICK_PLAN.md). Esta acta **distingue código implementado de capacidad probada**, especialmente porque Google Cloud no está disponible en esta etapa.

## Estado de la planificación

| Fase | Estado | Implementación concreta | Pendientes de aceptación |
|---|---|---|---|
| **D8-A — Identidad y UX** | **Implementación principal realizada; QA final pendiente** | Client ID público embebible en el binario Rust por `AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID`; ausencia detectada sin intento OAuth roto; formulario técnico solo para builds debug; UX «Conectar con Google», último backup, estado de cuenta y migración explícita si cambia la identidad | Client ID oficial, prueba OAuth real, revisión de UX en WebView2 y migración entre OAuth Client IDs |
| **D8-B — Cifrado automático** | **Parcial funcional** | Con consentimiento explícito el usuario puede guardar su frase en Windows Credential Manager (protección del perfil Windows) y AutoTrace la reutiliza para cifrar después de reinicios; confirmación doble; lectura de copias age anteriores; desactivación y eliminación de credencial | Kit de recuperación portable guiado, pruebas Windows de pérdida/cambio de perfil, tratamiento de frase perdida, esquema de clave por instalación con doble envoltura DPAPI y frase portable si se decide cambiar el formato |
| **D8-C — Robustez sin Google Cloud** | **Parcial** | Cola cifrada, uploads reanudables existentes; límite de 50 pendientes; verificación remota tamaño+MD5; categorías persistidas sin registrar respuestas confidenciales; 308 sin `Range` reintentable; mocks HTTP locales de 401/403/429/500 y JSON inválido; pruebas estáticas de conexiones | Refactor a adaptadores inyectables, mock completo de carga reanudable, jitter/backoff fino, fallas de disco/descarga, reintentos tras reinicio y ensayo exhaustivo de no duplicación |
| **D8-D — Google real** | **Bloqueada externamente** | Flujo OAuth Desktop PKCE y Drive API v3 ya en la base del PR #10; el binario puede recibir Client ID público por variable de repositorio, no un secreto | Crear/publicar una única aplicación OAuth de AutoTrace; habilitar Drive API; autenticación real, cancelación, refresh/revocación, subida y restauración en otra PC |
| **D8-E — Distribución** | **Parcial** | Pipeline NSIS Windows de compilación/test/install/uninstall; nuevos tests D8; rama independiente, sin merge | CI verde de SHA final, instalador de aceptación, WebView2 manual 100/125/150%, prueba en instalación limpia sin archivos de origen, kit de recuperación, eventual firma |

## Entregables reales de D8 en PR #11

1. Backend `desktop/src-tauri/src/drive_api.rs`: identidad OAuth `option_env!` pública, estados `ready`, `not_configured` y `migration_required`; modo de configuración manual restringido a debug; estados de sincronización `reconnect_required`, `rate_limited`, `quota_or_permission`, `provider_unavailable`, `integrity_failed`, `key_required`, `retry_required`, `remote_verified`. No se persisten tokens ni mensajes externos como diagnósticos.
2. Cifrado `desktop/src-tauri/src/drive_backup.rs`: acceso a frase age protegida por Windows, opcional y explícito, sin modificar el formato age existente ni la clave portable. Un cliente recuperado sin perfil Windows requiere **la frase original**. No confundir con recuperación automática de una contraseña olvidada.
3. Transporte `desktop/src-tauri/src/drive_transport.rs`: aceptación prudente de respuesta 308 sin rango confirmado y mocks HTTP locales sintéticos para errores de Google; jamás se borra una copia de cola antes de confirmar bytes remotos.
4. UX `site/app.js`: estado honesto sin Google configurado; CTA de conectar solo si hay identidad válida; modo anterior de carpeta sincronizada dentro de opciones avanzadas; aviso si un respaldo remoto verificado lleva más de 24 h sin actualizarse; protección opcional en Windows con frase duplicada y confirmación.
5. CI Windows: los workflows `desktop-foundation.yml` y `desktop-installer.yml` ahora leen `vars.AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID` como **variable pública de build**; si está ausente la PWA/SQLite permanece útil. No incluir secretos de cliente OAuth.
6. Tests: Rust para identidad, error de sincronización y rangos de subida; servidor HTTP local de prueba para 401/403/429/500 y JSON inválido; Node `tests/desktop-drive-oneclick.test.mjs`, browser smoke de PWA. **Sin afirmación de integración Google validada.**

## Pruebas a repetir en el SHA final

- Node `node --test tests/*.test.mjs`; Browser Chromium LAB y frontend sin Tauri.
- Windows `cargo test --manifest-path desktop/src-tauri/Cargo.toml --all-targets`; confirmar tests de Drive API.
- Compilación optimizada NSIS, instalación y desinstalación silenciosa en runner efímero, sin credenciales.
- **Sin Google Cloud**: verificar que la versión release muestra «integración todavía no activada» en vez de solicitar Client ID; respaldo SQLite/Excel local sigue funcionando.
- **Con Google habilitado más adelante**: build con Client ID público, autorización, red caída, refresh, cola, descarga y recuperación real en PC independiente. No validar con datos productivos inicialmente.

## Acciones del responsable

**No se requiere ninguna acción en Google Cloud ahora.** Cuando se destrabe, registrar **una sola identidad OAuth Desktop de AutoTrace** y añadir su **Client ID público** como variable de repositorio `AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID`; volver a construir el instalador y validar con una cuenta de prueba del taller. No subir refresh tokens, contraseñas ni backups reales a Actions.

## Limitación deliberada

La opción D8-B implementada guarda la **frase age original protegida por Windows** (solo con consentimiento), no aún una jerarquía criptográfica con clave de datos aleatoria por instalación. Esto resuelve la fricción de reinicio sin invalidar archivos antiguos; una mejora de doble envoltura requiere diseño de formato, migración y QA adicional. Si un tercero accede a la sesión Windows del usuario, la protección local no sustituye a cifrado de disco, bloqueo de sesión y controles de Windows.

**Cierre:** ningún gate D8-D/E de recuperación de otro equipo puede considerarse superado por compilación o mocks. Mantener PR #11 Draft.
