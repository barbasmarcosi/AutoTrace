# D7 — Google Drive API directa (sin Drive para escritorio)

**Estado:** PR #10 en borrador. Implementación de código destinada a AutoTrace **Desktop Windows**, no PWA Pages. El primer inicio de sesión real no puede completarse sin un OAuth Client ID de Google Cloud y aprobación de la cuenta del taller.

## Arquitectura

- **Sin servidores propios:** la app Tauri mantiene SQLite como fuente primaria y respaldos dobles SQLite + XLSX + manifiesto SHA-256 en la carpeta local privada. La funcionalidad Drive nueva se ejecuta solamente desde Desktop.
- **Antes de subir:** el paquete de respaldo dual se verifica y cifra con **age** y frase de recuperación definida por el usuario. Solo se transmite `AutoTrace_*.atbackup.age` (nunca SQLite, XLSX, contactos ni clave en claro).
- **Autorización:** OAuth 2.0 **Desktop Public Client** con PKCE SHA-256, navegador del sistema y callback localhost `127.0.0.1`; `state` validado. Solicita exclusivamente `https://www.googleapis.com/auth/drive.file`.
- **Credenciales:** Client ID (público) se guarda en la configuración de escritorio. Refresh token en **Windows Credential Manager**, sin archivo JSON ni GitHub Secrets. Google maneja la cuenta: AutoTrace nunca ve la contraseña de Google.
- **Cifrado:** la frase `age` no se guarda en disco, se desbloquea cada vez que se abre la aplicación. Guardala fuera de la PC. No se puede descifrar una copia si se pierde.
- **Subida:** Google Drive REST v3 `files.create` con `uploadType=resumable` y hasta 8 MiB por fragmento. Si la red se interrumpe, se reintenta la sesión. Si falla, permanece una cola local de archivos **ya cifrados**; sin conexión se reintenta cada 15 minutos mientras la app está abierta. No se sobrescriben archivos remotos.
- **Confirmación real:** después de la carga se verifica por la API `size`, `md5Checksum` y propiedad de origen del archivo. Solo entonces se actualizan los indicadores de subida verificada y se elimina el archivo de la cola. MD5 aquí es un control de transporte; la autenticidad de contenido y coherencia interna las aporta age + SHA-256/SQLite/Excel.
- **Carpeta remota visible:** se crea `AutoTrace Backups` con propiedad de aplicación; `drive.file` solo permite administrar archivos creados/autorizados por esta aplicación.
- **Recuperación:** Ajustes → Ver copias en Drive → Probar copia remota (sin modificar base) o Recuperar desde Drive (confirmación, backup local previo, reemplazo validado). La descarga verifica tamaño y MD5 antes de descifrar age y revisar SQLite/Excel.
- **Distintos destinos:** la función anterior que replica archivos cifrados en carpeta sincronizada sigue siendo opcional; la conexión API directa no depende de ella. El respaldo local nunca debe guardarse sin cifrar en una carpeta compartida.

## Evolución a experiencia en un clic (D8)

Este documento describe el **estado técnico actual del PR #10**, donde se introduce un Client ID de prueba desde Ajustes. El **producto final no mostrará ese campo al taller**: AutoTrace integrará el Client ID público de una única aplicación OAuth oficial y presentará solamente **Conectar con Google**. No se debe registrar un OAuth Client ID por cliente ni incrustar refresh tokens. El plan detallado, orden ejecutable sin Google Cloud, tratamiento de clave de recuperación y aceptación están en [D8 — Google Drive en un clic](D8_GOOGLE_DRIVE_ONE_CLICK_PLAN.md).

**Bloqueo conocido:** la versión D7 requiere desbloquear manualmente la frase age en cada sesión. Por eso no debe describirse como copia remota completamente desatendida hasta implementar D8-B (clave de cifrado protegida por Windows y mecanismo independiente de recuperación). También se debe contemplar la migración entre distintos OAuth Client IDs, que no asegura acceso transparente a todos los archivos creados por una aplicación anterior.

## Activación — configuración externa que necesita el propietario

1. Abrir [Google Cloud Console](https://console.cloud.google.com/), crear/elegir un proyecto y **habilitar Google Drive API** para él.
2. Abrir [Google Auth Platform](https://console.cloud.google.com/auth/overview) y configurar el nombre de AutoTrace y la pantalla de consentimiento de Google OAuth. Si se usa en modo pruebas, agregar las cuentas autorizadas como usuarios de prueba. Para distribuir a clientes externos, completar los requisitos de publicación/verificación aplicables.
3. En **Clientes OAuth**, crear uno de tipo **Desktop app** (no Web app). Copiar solamente el **Client ID**, que termina en `.apps.googleusercontent.com`. AutoTrace usa un cliente público con PKCE y no necesita almacenar un client secret.
4. Instalar la **próxima versión Desktop que incluya el PR #10**. En Ajustes → Google Drive · Conexión directa → Configurar Client ID, pegar el ID y pulsar **Conectar Google**. Elegir la cuenta del taller y aceptar permiso para archivos creados por AutoTrace.
5. Pulsar **Desbloquear cifrado** e ingresar la frase de recuperación (16–256 caracteres); si ya existía una cola cifrada, debe poder descifrarse con la misma frase.
6. Pulsar **Crear y subir copia ahora** y verificar la fecha de la **última subida verificada**. El archivo debe aparecer en `AutoTrace Backups` de la cuenta autorizada.
7. Hacer la prueba crítica desde otra computadora: instalar Desktop, registrar el Client ID, autorizar la misma cuenta, listar las copias remotas y ejecutar **Probar copia remota** con la frase de recuperación, sin acceso a los archivos de la primera PC.

## Errores, límites y seguridad

- La integración no puede conectarse en este repositorio sin que exista un Client ID válido y consentimiento de una cuenta Google: nunca se inventan tokens.
- En modo de prueba externo, Google puede limitar usuarios y caducidad de refresh tokens. Para entregar una app general al público corresponde completar el proceso de publicación OAuth.
- No almacenar ni subir contraseñas, refresh tokens, client secrets o respaldos reales en GitHub, Actions ni el código de frontend.
- La sincronización funciona **mientras AutoTrace Desktop está abierto** y la PC tiene red. PC apagada o app cerrada no genera snapshots nuevos; copias cifradas pendientes se reintentan al volver a abrir.
- **No hay borrado automático de copias remotas** en esta fase: evita pérdida accidental. La retención de Drive y la política de cuotas requieren una etapa posterior segura.
- No hay promesas de almacenamiento gratuito ilimitado ni disponibilidad de Google Cloud/Drive. Los respaldos consumen cuota de la cuenta.
- Si el backend reporta éxito de copia local y fallo cloud, **los datos actuales no se pierden**. La app distingue `cloudCopyOk` de `driveCopyOk` (carpeta sincronizada anterior).
- La clave se conserva solamente en RAM durante la sesión, pero **no está ligada a la identidad Google**; si cambiás de cuenta con archivos pendientes, comprobar qué copias se transferirán.
- Los archivos locales SQLite y Excel aún se generan sin cifrar en el almacenamiento privado local de AutoTrace: mantené la protección del usuario Windows y cifrado del disco (BitLocker) si corresponde.
- La integridad descargada debe ser probada y no depende de que el archivo figure en un listado.

## Gates para cerrar PR y producir otro instalador

- Tests Rust Windows / Node / Chromium exitosos sobre HEAD.
- OAuth real con usuario de prueba: conectar, refrescar token desde Windows Credential Manager tras reiniciar, revocar acceso, reconectar.
- Subida resumable real, listado, red cortada y recuperación tras reconexión.
- Descarga y recuperación **desde otra PC limpia**; validación de datos, cobros, cuentas y fechas.
- Empaquetar y comprobar instalador Windows en runner limpio (el instalador anterior **NO incluye** conexión API directa).
- Revisión de seguridad del consentimiento OAuth, publicación y estado de cuotas.

Fuentes de diseño: [Google OAuth Desktop](https://developers.google.com/identity/protocols/oauth2/native-app), [Drive upload](https://developers.google.com/workspace/drive/api/guides/manage-uploads), [Drive file permissions](https://developers.google.com/workspace/drive/api/guides/api-specific-auth).
