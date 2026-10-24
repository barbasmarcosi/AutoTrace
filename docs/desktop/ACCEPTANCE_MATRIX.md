# Matriz de aceptación — AutoTrace Desktop / XLSX / recuperación

**Estado:** pruebas por implementar; casillas vacías son explícitamente pendientes. Ningún caso se declara PASS por aparecer en esta tabla.

## A. Persistencia y compatibilidad

| ID | Prueba observable | Gate |
|---|---|---|
| D-A01 | Instalar y abrir sin internet; crear cliente, vehículo, trabajo; reiniciar y consultar mismos IDs/campos | Obligatorio |
| D-A02 | Alta/edición/baja segura respeta FK y unicidad de patente; duplicado devuelve error legible | Obligatorio |
| D-A03 | `charged`, `paid`, `partsCost`, `laborCost` preservan centavos y `paid <= charged` | Obligatorio |
| D-A04 | JSON producido por PWA actual migra sin cambiar conteos, IDs, relaciones, estados ni demo flag | Obligatorio |
| D-A05 | Migración fallida: cero filas parciales y JSON de origen intacto | Obligatorio |
| D-A06 | Segunda importación del mismo contenido no genera duplicados ni silencios; confirmación reemplazo explícita | Obligatorio |
| D-A07 | Pages (PWA IndexedDB) sigue funcionando y desplegándose sin dependencias desktop | Obligatorio |

## B. Excel con vuelta completa

| ID | Prueba observable | Gate |
|---|---|---|
| D-X01 | Exportar `Clientes`, `Vehiculos`, `Trabajos`, `Configuracion`, `Manifest` con IDs inalterados | Obligatorio |
| D-X02 | SQLite A → XLSX → SQLite B produce igualdad semántica canónica, incluso Unicode y ceros iniciales | Obligatorio |
| D-X03 | Rechazar `client_report` como backup; exportación «para cliente» no revela costos internos/notas | Obligatorio |
| D-X04 | Libro editado: el modo «restaurar verificado» rechaza checksum distinto; modo «importar editado» advierte y valida | Obligatorio |
| D-X05 | Celdas con fórmulas, enlaces peligrosos, valores negativos, fecha imposible y FK huérfana rechazan con ubicación | Obligatorio |
| D-X06 | Libro truncado/ZIP con límites excedidos se rechaza sin tocar BD | Obligatorio |
| D-X07 | Cancelar preview no modifica datos; restaurar genera copia pre-restore | Obligatorio |
| D-X08 | Sobrescribir mismo destino manual requiere confirmación nativa; el automotor nunca pisa backup previo | Obligatorio |
| D-X09 | `paid` acumulado se preserva sin inventar registros de pago fechados | Obligatorio |

## C. Respaldo automático y recuperación

| ID | Prueba observable | Gate |
|---|---|---|
| D-B01 | Arranque con copia atrasada dispara nuevo backup mientras app está abierta | Obligatorio |
| D-B02 | Dos solicitudes concurrentes producen un único snapshot coherente y archivos con nombres únicos | Obligatorio |
| D-B03 | SQLite y XLSX parten del mismo snapshot; hashes/manifiesto y conteos coherentes | Obligatorio |
| D-B04 | Corte en mitad de XLSX no declara copia exitosa ni destruye la anterior | Obligatorio |
| D-B05 | `integrity_check`, `foreign_key_check`, relectura XLSX y checksums verifican antes de éxito | Obligatorio |
| D-B06 | USB removido / disco lleno / carpeta inaccesible: error visible, retry posterior, sin pérdida de datos | Obligatorio |
| D-B07 | Retención jamás borra última copia válida, ni archivos no reconocidos en destino | Obligatorio |
| D-B08 | Restauración desde una unidad externa **en PC de prueba limpia y sin red** recupera datos | Obligatorio |
| D-B09 | Nube solo se considera destino externo al verificar artefacto cifrado y recuperación; no confiar en icono sync | Opcional D6 |
| D-B10 | Copia con app cerrada solo si Task Scheduler/servicio complementario está **instalado y probado** | Opcional D6 |
| D-B11 | Restauración abortada preserva base actual y copia previa sin corrupción | Obligatorio |
| D-B12 | Archivo `.atbackup.age` no expone datos, cifrado con frase fuerte; contraseña errónea y manipulación fallan sin tocar datos actuales | D6 obligatorio para nube |
| D-B13 | Nueva PC sin originales locales importa backup descargado de Google Drive y recupera SQLite, XLSX, cuentas y cobros | D6 obligatorio para nube |
| D-B14 | Drive apagado/sin conexión/carpeta no disponible informa que solo hay copia local; **no** declara subida remota confirmada | D6 obligatorio para nube |
| D-B15 | Al reiniciar se requiere desbloquear clave; no guardar frase de recuperación en disco, configuraciones, CI ni logs | D6 obligatorio para nube |

## C.1. Drive API directa (D7)

| ID | Prueba observable | Gate |
|---|---|---|
| D-G01 | Consentimiento OAuth Desktop en navegador real con PKCE S256, `state` y scope solo `drive.file` | Obligatorio D7 |
| D-G02 | Token de renovación exclusivamente en Windows Credential Manager; sobreviven reinicio y desconexión, se eliminan al desconectar | Obligatorio D7 |
| D-G03 | No se envía ningún SQLite/Excel sin cifrar: carga age, verificada con `size`, MD5 y origen de aplicación mediante Drive API | Obligatorio D7 |
| D-G04 | Sin red se conserva cola cifrada; se reintenta sin duplicar ni sobrescribir archivos remotos | Obligatorio D7 |
| D-G05 | Otra PC y otra instalación pueden listar y recuperar copias de la misma cuenta Google con el mismo OAuth Client ID | Obligatorio D7 |
| D-G06 | Archivo manipulado, frase equivocada o backup truncado: importación rechazada, base vigente preservada | Obligatorio D7 |
| D-G07 | Indicar por separado backup local generado, copia cloud pendiente y cloud confirmada; no simular éxito por solo entrar en Drive | Obligatorio D7 |

## C.2. Experiencia en un clic y cifrado autónomo (D8)

| ID | Prueba observable | Gate |
|---|---|---|
| D-H01 | Build final trae el Client ID **público** de AutoTrace; usuario del taller nunca ve ni completa Client ID/Google Cloud | Obligatorio D8 |
| D-H02 | Build de desarrollo sin identidad oficial informa «no configurado» sin romper SQLite, backups ni PWA | Obligatorio D8 |
| D-H03 | Después de reiniciar Windows, AutoTrace cifra nuevos respaldos con clave protegida por el sistema; no necesita pedir la frase cada vez | Obligatorio D8 |
| D-H04 | Con pérdida total de PC y otra identidad Windows, cuenta Google + kit portable permiten descifrar copia y restaurar sin datos locales previos | Obligatorio D8 |
| D-H05 | Respuesta perdida tras subir, 308, 401, 403, 429, 5xx, desconexión y espacio insuficiente se prueban con adaptador HTTP simulado; sin duplicados ni pérdida | Obligatorio D8 |
| D-H06 | Token vencido/revocado no se presenta como «conectado y respaldando»; UI indica reconexión, última copia verificada y cola pendiente | Obligatorio D8 |
| D-H07 | Cambio de OAuth Client ID no causa pérdida silenciosa: se ofrece exportación de `.atbackup.age` y migración explícita de archivos históricos | Obligatorio D8 |
| D-H08 | Release sin OAuth autorizado mantiene función local intacta; no se distribuyen refresh tokens, passwords, client secrets ni datos de usuarios | Obligatorio D8 |
| D-H09 | Sincronización con aplicación cerrada/PC apagada NO se promete hasta implementar y probar un servicio o Programador de tareas | Gate de honestidad |

Ver planificación y dependencias: [D8 — Google Drive en un clic](D8_GOOGLE_DRIVE_ONE_CLICK_PLAN.md).

## D. Experiencia, build y soporte

| ID | Prueba observable | Gate |
|---|---|---|
| D-U01 | Indicador de salud cambia entre actualizado, solo local, atrasado y error; explica cómo resolverlo | Obligatorio |
| D-U02 | Importación preview muestra qué será reemplazado y exige confirmación explícita | Obligatorio |
| D-U03 | Generar instalador Windows y ejecutar instalación limpia, upgrade y desinstalación (datos preservados por defecto) | Obligatorio |
| D-U04 | Interfaz sin scroll horizontal, menús seleccionables y rutas correctas en Windows WebView2 a 100/125/150% DPI | Obligatorio |
| D-U05 | Sin secretos, datos de clientes ni backups de producción en repositorio/artefactos Actions | Obligatorio |
| D-U06 | Manual de recuperación legible y probado por persona distinta de quien implementó | Obligatorio |
| D-U07 | Firma de código Windows y advertencias SmartScreen claramente documentadas si no se firma | Recomendado |

## Datos sintéticos mínimos

- 5 clientes (uno sin vehículos), 7 vehículos (dos del mismo propietario), 12 órdenes (todos los estados), patente `AF 012 BC`, teléfono con cero inicial, cadenas con tilde/emoji/apóstrofe, notas privadas, importes con centavos y saldos 0/parciales, un vehículo sin órdenes.
- Valores inválidos aislados: ID duplicado, patente duplicada, referencia huérfana, pago mayor a lo cobrado, fecha 2026-02-31, celda `=HYPERLINK(...)`, libro sin Manifest, macro `.xlsm`, EXCEL ZIP alto ratio.
- Nunca usar datos verdaderos del cliente en fixtures ni screenshots públicas.

## Política de cierre

Cada gate obligatorio requiere evidencia concreta: comando, plataforma, commit SHA, fixture, resultado y log sin secretos. **`BUILD PASS` no equivale a `RECOVERY PASS`.** Mantener PR de implementación en Draft hasta cumplir migración, backup y restauración, y solo entonces considerar despliegue al equipo del cliente.
