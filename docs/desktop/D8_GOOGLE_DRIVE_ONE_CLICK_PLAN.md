# D8 — Google Drive en un clic: plan ejecutable sin bloquear el desarrollo en Google Cloud

**Fecha de planificación:** 2026-10-24  
**Base técnica:** PR #10 `feature/drive-api-oauth-direct-20261023` (Draft), apilado sobre PR #9 → #8 → #7.  
**Objetivo del producto:** el taller instala AutoTrace Desktop, pulsa **Conectar con Google**, autoriza su cuenta y AutoTrace cifra/sube respaldos automáticamente, con estado verificable y recuperación en otra PC. **Ningún usuario final configura Google Cloud, Client ID ni API.**

## 0. Decisiones cerradas

1. **AutoTrace Desktop sigue siendo local-first.** SQLite es la fuente primaria; Google Drive solo transporta copias cifradas. Sin servidor obligatorio ni cuenta AutoTrace.
2. **Una sola aplicación OAuth para el producto.** Registrar **una** identidad OAuth Google Cloud `Desktop app` administrada por AutoTrace, y distribuir su **Client ID público** en el instalador. Nunca incluir un `client_secret`, refresh token ni sesión personal en el instalador, frontend, repositorio o CI. Los clientes solo autorizan su propia cuenta.
3. **Permisos mínimos:** scope `https://www.googleapis.com/auth/drive.file`; sin lectura general de Drive. Archivos propios bajo `AutoTrace Backups`, sin permisos públicos, sin compartir automáticamente.
4. **OAuth Desktop seguro:** navegador del sistema, PKCE S256, callback loopback, `state`, token de actualización en almacén de credenciales de Windows. Si falla, el backup local continúa.
5. **Cifrado antes de enviar:** paquete age autenticado con SQLite + Excel + manifest SHA-256; sin datos personales de clientes en la telemetría ni respuestas de UI. El cifrado protege incluso ante alguien con acceso a Drive.
6. **Copias locales y copias remotas son estados distintos:** `LOCAL_VERIFIED`, `ENCRYPTED_PENDING`, `UPLOADING`, `REMOTE_VERIFIED`, `RESTORE_TESTED`, `ACTION_REQUIRED`. Una carga `200` no basta sin comparar tamaño y checksum retornados por Drive.
7. **No declarar terminado hasta un simulacro en otra PC**, usando un archivo realmente descargado de la nube. Compilar e instalar no equivale a recuperar.
8. **No tocar la PWA Pages estable ni fusionar PR #7–#10** hasta satisfacer los gates y resolver la cadena de PR. El instalador de prueba 0.1.1 no equivale a publicación productiva.
9. **Google Cloud queda fuera del camino crítico por ahora.** Todo lo que no necesita OAuth real se implementa y comprueba con dobles de prueba locales; activación real queda como un único gate de integración cuando la cuenta administradora esté disponible.

## 1. Experiencia del cliente final (UX contractual)

Pantalla **Ajustes → Copias de seguridad**:

**Sin cuenta conectada**
- Encabezado: «Protegé los datos de tu taller».
- CTA primario: **Conectar con Google** (sin Client ID visible).
- Mensaje: «Tus respaldos se cifran antes de enviarse a tu Google Drive. No necesitamos acceso al resto de tus archivos».
- En builds de desarrollo sin identidad oficial: deshabilitar el CTA e indicar **«La integración todavía no está activada en esta edición de prueba»**. No abrir un flujo destinado a fallar. Mostrar Client ID solo en **configuración avanzada de desarrollador**, no en UX de cliente.

**Primer uso autorizado**
1. Clic **Conectar con Google** → navegador del sistema → elegir cuenta → consentimiento Google.
2. Regreso automático a AutoTrace: cuenta conectada; identificarla con etiqueta segura de cuenta si está autorizado el dato, o «Google Drive conectado» sin pedir `userinfo.email` por defecto.
3. Configurar o recuperar **clave de recuperación de backups**. Mostrar la advertencia «Sin esta clave no podremos restaurar en otra computadora» y exigir una confirmación o ensayo. No guardar la frase en claro.
4. Generar primer snapshot; estado **Respaldando / Cifrando / Subiendo / Verificado en Drive**, con fecha UTC formateada localmente y botón «Probar recuperación».
5. Próximos usos: generar y subir copias automáticamente mientras AutoTrace está abierto; si no hay Internet, conservar la cola cifrada y reintentar sin pedir intervención; si perdió autorización, mostrar «Reconectar Google».

**Recuperación en PC nueva**
1. Instalar AutoTrace Desktop → **Recuperar desde Google Drive**.
2. Conectar la misma cuenta Google de Drive (OAuth de AutoTrace, sin configuración técnica).
3. Listar copias con fecha, tamaño, identificador y verificación; recomendar la última **que haya pasado simulacro**.
4. Descarga en área temporal, verificación remota + age + manifest + doble reconstrucción SQLite/XLSX.
5. Vista previa de clientes/vehículos/trabajos/cobros/cuentas; confirmar reemplazo; crear copia del estado actual antes de guardar; limpiar temporales.
6. Un fallo no modifica la base actual. Poder regresar a una copia anterior sin eliminar las posteriores.

**Soporte para versiones anteriores:** conservar el modo de carpeta sincronizada existente durante migración, pero colocarlo en «Opciones avanzadas → Método anterior»; no confundirlo con la conexión directa.

## 2. Auditoría técnica y tareas — en orden de implementación

### Fase D8-A · Identidad embebida y UI sin configuraciones técnicas (SIN Google Cloud)

**Archivos candidatos:** `desktop/src-tauri/src/drive_api.rs`, `desktop/src-tauri/src/drive_oauth.rs`, `site/app.js`, `desktop/src-tauri/tauri.conf.json`, `.github/workflows/desktop-installer.yml`.

- [ ] Inyectar `AUTOTRACE_GOOGLE_OAUTH_CLIENT_ID` **en tiempo de compilación** como cadena **pública** (por ejemplo, `option_env!` en Rust). No tratarla como credencial sensible. Fallback a Client ID manual **solo en modo desarrollo**; la versión para taller jamás muestra ese campo.
- [ ] Definir contrato `oauthIdentityStatus`: `ready`, `not_configured`, `invalid`; CTA solo habilitado cuando `ready`.
- [ ] Simplificar la pantalla: una sola sección «Copias de seguridad», CTA y semáforo de salud; mantener Configuración avanzada para antiguo método.
- [ ] Los comandos no configurados fallan con error amigable y **no solicitan ni almacenan** tokens. La app nunca representa «conectado» solo porque existe un refresh token si Google lo revocó.
- [ ] Añadir pruebas Rust del Client ID empacado/malformado y pruebas de interfaz para ambos estados sin hacer solicitudes a Google.
- [ ] Probar la instalación sin identidad OAuth: AutoTrace sigue iniciando, opera offline, genera backup local y permite exportar/importar.
- [ ] CI verifica ausencia de client_secret, refresh token, respaldo real y `drive.file` expandido accidentalmente.

**Aceptación:** el usuario normal no ve ningún formulario técnico; con identidad ausente hay explicación clara, no un botón roto.

### Fase D8-B · Cifrado automático sin pedir la frase en cada sesión (SIN Google Cloud)

**Deuda detectada en PR #10:** la copia remota necesita que el operador vuelva a desbloquear manualmente age tras reiniciar. Así, aunque OAuth esté conectado, **no podemos prometer backups automáticos desatendidos**.

- [ ] Diseñar y versionar una envoltura de claves: clave aleatoria por instalación/bóveda que cifra age; **envoltura local con Windows DPAPI** y envoltura de recuperación portable ligada a la frase/clave del taller. No almacenar la contraseña portable en claro.
- [ ] En el primer uso presentar **kit de recuperación** (clave portable, instrucciones en papel y archivo cifrado de prueba); validar que se pueda usar en una PC nueva.
- [ ] Soportar fallos de DPAPI, cambio de usuario Windows, migración de PC y pérdida de datos del perfil Windows sin perder la copia en Drive (siempre con recuperación portable).
- [ ] Migrar **sin borrar ni sobrescribir** copias age del esquema actual; aceptar ambos formatos en lectura, versionar el nuevo encabezado y registrar migración.
- [ ] Probar arranque + cierre + reinicio: la copia automática funciona usando solo secretos sellados por Windows, sin que el usuario ingrese la frase cada día; la clave portable queda fuera de la PC.
- [ ] Prohibir guardar el kit de recuperación sin cifrar en la misma carpeta de Google Drive.

**Aceptación:** backup cifrado automático real durante la sesión de Windows sin intervención repetida; pérdida completa de la PC reversible con cuenta Google + kit de recuperación.

### Fase D8-C · Transporte seguro y resiliencia (mock local, SIN Google Cloud)

**Deudas a examinar del PR #10:** manejo de reintentos, idempotencia de uploads, expiración/revocación OAuth, paginación y control del almacenamiento.

- [ ] Extraer `DriveTransport` y `OAuthProvider` detrás de interfaces/adaptadores Rust para poder probar respuestas HTTP sin abrir Google Cloud.
- [ ] Servidor HTTP de pruebas **local** que simula: token OAuth vencido/revocado, `401`, `403`, `429`, `5xx`, `308 Range`, red desconectada, subida parcialmente confirmada, paginación, checksum erróneo y JSON malformado.
- [ ] Reintentos con backoff + jitter y límite, sincronización no bloqueante para UI, estado persistente de cola cifrada y snapshot inmune a interrupción. Recuperar `session URI` de subida solo mientras sea válida y evitar exponerla a logs.
- [ ] Idempotencia **por identificador y checksum**, incluso después de perder respuesta HTTP; no subir dos copias distintas con mismo ID.
- [ ] No declarar «REMOTE_VERIFIED» sin verificar `size` + `md5Checksum` **contra bytes exactos del archivo** + atributos de AutoTrace.
- [ ] Si Drive devuelve 401/token inválido: estado `RECONNECT_REQUIRED`; jamás borrar copias locales pendientes.
- [ ] Registrar solo eventos no sensibles (estado, ID aleatorio, conteos, horas); no tokens, URLs OAuth, rutas internas completas, nombres de cliente ni contenido de respaldos.
- [ ] Configurar retención **no destructiva** primero (mostrar ocupación y antigüedad); no implementar borrado automático remoto hasta disponer de pruebas de recuperación, política versionada y consentimiento.
- [ ] Definir cuota local de pendientes, disco lleno, réplica secundaria y alertas. Si Drive falla, AutoTrace continúa trabajando.
- [ ] Añadir pruebas negativas de restauración parcial, corrupción, archivo ajeno a AutoTrace y error a mitad de descarga.

**Aceptación:** matriz de errores reproducible en CI sin Google real; un corte de red no destruye ni duplica respaldos.

### Fase D8-D · Activación OAuth real (BLOQUEADA HASTA QUE GOOGLE CLOUD FUNCIONE)

**Operación administrativa única, no por cliente** (no insistir al usuario hasta este gate):
1. Proyecto de Google Cloud administrado por el producto AutoTrace y habilitación de Google Drive API.
2. Pantalla de consentimiento/Google Auth Platform con nombre, enlaces y políticas requeridas, cuentas de prueba y publicación apropiada.
3. Crear **OAuth Client ID de tipo Desktop**; proporcionar **solo el identificador público** al build. Nunca almacenar en GitHub el refresh token de un usuario real. Rotación y proceso de renovación de identidad documentados.
4. Verificar la política de scopes; `drive.file` es de acceso por archivo y categoría no sensible, pero la publicación/validación de la aplicación puede requerir pasos administrativos.
5. No utilizar indefinidamente el estado externo **Testing**: según Google, los refresh tokens de aplicaciones externas en ese estado normalmente **vencen a los 7 días** para scopes como Drive. No vender como «conectar una sola vez» mientras siga Testing.
6. Pruebas manuales: consentimiento, cancelación, reconexión, revocación, reinicio, subida, descarga, alteración, 2 PCs, fallo de Internet, 2 cuentas distintas.
7. Compilar instalador firmado cuando sea viable; si no hay certificado, identificarlo claramente como preview unsigned (advertencia de SmartScreen).

**Migración entre Client IDs:** los permisos `drive.file` están vinculados a la aplicación autorizada; **no asumir que un Client ID de prueba y otro productivo verán los mismos archivos**. Exigir conservar copia age descargada/exportada y definir migración explícita; nunca cambiar identidad en silencio si existe una cola cifrada.

**Aceptación:** instalador con OAuth público empacado, sin formularios técnicos para taller y prueba de recuperación desde Drive en equipo distinto. Recién entonces considerar merge/release.

### Fase D8-E · Aceptación comercial y operativa

- [ ] Un operador no técnico completa conexión, primera copia y primer simulacro con instrucciones normales, sin Google Cloud.
- [ ] En una PC sin archivos previos, inicia sesión, usa el kit y recupera **exactamente** clientes, vehículos, órdenes, detalles, cobros, métodos, saldos, cuentas, preferencias y datos históricos.
- [ ] Alertar si pasan 24 h sin snapshot **remoto verificado** (plazo configurable; no prometer copias con PC apagada).
- [ ] Documentar que sincronizar respaldos no es sincronización multiusuario/multi-PC simultánea. Una sola PC activa escribe la base; otra se usa para recuperación o migración controlada.
- [ ] Ensayo de restauración mensual no destructivo; política de retención (últimas N / diarias / semanales / mensuales) solo después del ensayo exitoso.
- [ ] Evaluar qué ocurre cuando el técnico desinstala, Windows reemplaza perfil, Google alcanza su cuota, pierde acceso a cuenta o pierde el kit de recuperación.
- [ ] No merge sin informes de pruebas y aprobación del responsable de entrega. 

## 3. Backlog priorizado (para trabajo independiente de Google)

| Orden | Entrega | Alcance | Dependencia | Resultado requerido |
|---|---|---|---|---|
| P0 | D8-A identidad empacable + UX un clic | Rust + frontend + tests | Ninguna | El usuario no ve Client ID |
| P0 | D8-B desbloqueo seguro automático | DPAPI + clave portable + compatibilidad | D8-A | Copias cifradas al reiniciar |
| P0 | D8-C transporte con mocks de fallos | Rust + CI Windows | D8-A | Ninguna pérdida/duplicación |
| P0 | D8-D integración OAuth real | Consola Google administradora | Google Cloud disponible | Subida, descarga y recuperación real |
| P1 | D8-E instalador/distribución | NSIS + pruebas en PC limpia | D8-D | Vista previa de aceptación |
| P1 | Recuperación y retención guiadas | UI + política sin borrado prematuro | D8-B/C | Recuperación operativa |
| P2 | Servicio con AutoTrace cerrado | Scheduler de Windows cifrado | DPAPI + diseño operacional | Copias aún con app cerrada |
| P2 | Dispositivos múltiples | Política de autoridad y conflictos | Diseño aparte | No sobrescribir trabajos |

## 4. Evidencia mínima por gate

Registrar en `docs/desktop/` por cada etapa: SHA exacto, plataforma, comando, fixture **sintético**, resultados, ejecución Actions y limitaciones. Reusar la matriz `ACCEPTANCE_MATRIX.md`.

**No hacer:** fingir que «Google conectado» es backup correcto; usar scraping del formulario Drive; insertar Client ID de cuentas ajenas; sincronizar SQLite viva o XLSX sin cifrar; pedir a cada cliente que abra Google Cloud; usar un único kit de recuperación para todos los talleres.

## 5. Riesgos críticos a resolver antes de producción

1. **Pérdida del kit:** cifrado irrecuperable; no hay recuperación por soporte. Requerir confirmación de resguardo.
2. **OAuth Testing y cambio de Client ID:** pérdida de autorización y posible invisibilidad de backups anteriores, aunque sigan en Drive. Respaldos exportables independientemente de la API.
3. **Google sin servicio / cuota:** cola cifrada y backup local deben seguir funcionando; estado «última copia remota confirmada» no se actualiza falsamente.
4. **Compromiso de Windows:** proteger tokens locales, frase en memoria y datos SQLite/XLSX locales; recomendar BitLocker y cuenta Windows con contraseña.
5. **Colisión y borrado:** no sobreescribir snapshots, nunca limpiar toda la carpeta remota automáticamente.
6. **Fallos silenciosos:** notificación clara si el respaldo remoto deja de verificarse; registrar resultados sin datos privados.

## 6. Decisión de salida

**Ahora:** ejecutar D8-A/B/C y pruebas con mocks. Mantener PR #10 en Draft sin tocar la PWA estable.

**Cuando Google Cloud se destrabe:** activar D8-D una sola vez para todo AutoTrace, realizar recuperación en PC limpia y empaquetar la versión candidata. **No hace falta volver a diseñar ni pedirle Google Cloud a cada taller.**

Referencias de requisitos de proveedor:
- https://developers.google.com/identity/protocols/oauth2/native-app
- https://developers.google.com/identity/protocols/oauth2
- https://developers.google.com/workspace/drive/api/guides/api-specific-auth
- https://developers.google.com/workspace/drive/api/guides/manage-uploads
