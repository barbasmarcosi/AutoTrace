# D6 — Respaldo cifrado para carpetas sincronizadas con Google Drive (Desktop)

**Estado:** implementado en PR #8 (Draft); **no habilitado ni validado en una instalación de cliente**.

## Objetivo y límites

AutoTrace funciona sin un servidor ni APIs de Google. En Windows Desktop genera respaldos verificados SQLite + Excel + manifiesto y, opcionalmente, los empaqueta dentro de **un solo archivo cifrado `.atbackup.age`** en una carpeta elegida por el usuario, destinada a ser sincronizada por **Google Drive para escritorio**.

**Importante:** escribir un archivo dentro de la carpeta sincronizada **no demuestra que se haya subido a la nube**. AutoTrace muestra la última escritura local, y el usuario debe corroborar la sincronización en la interfaz de Google Drive. La PWA Pages no puede escribir periódicamente en carpetas arbitrarias ni abrir la base SQLite Desktop: esta funcionalidad es **exclusiva de la aplicación Windows**.

## Preparación en una PC

1. Instalar Google Drive para escritorio, iniciar sesión y elegir preferentemente el modo **Duplicar archivos (Mirror files)**, con carpeta local disponible.
2. En Drive crear una carpeta exclusiva `AutoTrace-cifrado`. No ubicar allí respaldos SQLite o Excel sin cifrar. Esperar a que Google Drive indique que la carpeta está sincronizada.
3. Abrir AutoTrace **Desktop**, entrar en **Ajustes → Google Drive · copia cifrada → Configurar carpeta cifrada**.
4. Definir y confirmar una **frase de recuperación de 16–256 caracteres**, preferentemente larga y única; guardarla en un soporte seguro **fuera de la computadora**. Si se pierde, no existe recuperación posible de esos archivos.
5. Elegir la carpeta configurada por Google Drive. AutoTrace verificará si ya contiene archivos propios y, si hay, exigirá que se puedan descifrar con la frase ingresada.
6. Se generará una nueva copia local y una réplica **cifrada** en esa carpeta. Corroborar el archivo `AutoTrace_<ID>.atbackup.age` desde la interfaz web de Drive, no solo en el Explorador de Windows.
7. Tras reiniciar AutoTrace Desktop, la frase se **vuelve a ingresar** usando **Desbloquear copias**. Si no se desbloquea, los backups locales habituales siguen ejecutándose, pero la réplica cifrada se informa como pendiente/error. La clave nunca se almacena en un archivo o en el repositorio.

Cada archivo `.age` contiene un único snapshot completo: SQLite, Excel y manifest. El lector verifica que las dos representaciones reproducen exactamente el mismo estado y que coincidan con sus SHA-256. El cifrado se realiza con el formato estándar **age**, en modo contraseña con scrypt y autenticación integrada. Se genera un archivo único por copia; la aplicación **no sobrescribe copias anteriores**.

## Prueba de recuperación sin modificar datos

En **Ajustes → Google Drive · copia cifrada → Probar copia cifrada** ingresá la frase de recuperación y seleccioná un archivo `.atbackup.age` que **hayas descargado realmente** desde Google Drive. AutoTrace lo descifra temporalmente, verifica hashes y relaciones, restaura sus datos de forma independiente en dos bases SQLite temporales (una usando el SQLite respaldado y otra importando el Excel), y compara ambos resultados.

La prueba muestra conteos de clientes, vehículos y trabajos. **No reemplaza ni modifica la base actual.** Si la frase está equivocada, el archivo está adulterado, faltan datos o Excel y SQLite difieren, falla. El directorio temporal descifrado se elimina al terminar.

**Esta función no comprueba que Google Drive haya sincronizado automáticamente**: para ensayar recuperación fuera del equipo de origen, elegí una copia efectivamente descargada desde la web de Drive y guardala en otra computadora.

## Recuperación en otra computadora

1. Instalar la misma versión compatible de AutoTrace Desktop; preparar una carpeta local propia para los datos de AutoTrace.
2. Desde Google Drive descargar un archivo `.atbackup.age` y verificar que terminó de descargarse por completo. No depender de un marcador en línea de un archivo no disponible sin red.
3. Abrir **Ajustes → Recuperar copia cifrada**; ingresar la frase de recuperación y seleccionar el archivo descargado.
4. AutoTrace descifra **a una carpeta temporal de uso exclusivo**, valida los SHA-256, SQLite y Excel, y presenta los conteos de clientes, vehículos y trabajos.
5. Confirmar el **reemplazo completo** solo después de revisar la vista previa. AutoTrace intenta crear una copia previa del estado actual antes de importar el backup; si falla, no reemplaza nada.
6. Verificar los clientes, vehículos, trabajos, saldos, cobros y cuentas restauradas. Configurar de nuevo la carpeta cifrada para seguir respaldando desde la computadora nueva.
7. Realizar periódicamente una recuperación real desde una copia descargada de Drive: no alcanza con un archivo que solo existe en la PC original.

## Garantías y restricciones de seguridad

- La carpeta sincronizada contiene solamente archivos **cifrados** creados por esta función. El backup **SQLite + XLSX local existente sigue sin cifrar** en la carpeta privada de datos de AutoTrace; la opción de copia externa anterior también es sin cifrar y debe tratarse por separado.
- AutoTrace **no almacena ni envía la contraseña a Google**. Mantiene la frase solo durante la sesión Desktop, en memoria. La frase no aparece en logs ni en archivos de configuración.
- No copiar jamás `.sqlite3`, `.xlsx` ni `.manifest.json` completos sin cifrar a una carpeta compartida de Google Drive.
- Google Drive sincroniza cambios y borrados; **no es un backup inmutable**. Preservar varias versiones y una copia externa desconectada, y evitar borrar desde Drive antes de un ensayo de recuperación. La réplica cifrada de esta fase no hace limpieza destructiva de copias antiguas.
- El nombre del archivo revela que es una copia AutoTrace y contiene una fecha/ID; su contenido interno (incluidos datos personales) está cifrado.
- No se prometen disponibilidad, cantidad de espacio, gratuidad garantizada, subida remota ni capacidad de trabajar con la PC apagada. La frecuencia automática actual depende de que AutoTrace Desktop esté funcionando y la frase esté desbloqueada.
- Si Drive no está instalado, el usuario puede **deshabilitar el destino** dejando de sincronizar esa carpeta; la app no realiza autenticación ni opera la API de Drive.
- En una carpeta Drive virtual con archivos solo en la nube podrían fallar escrituras atómicas/lecturas; **preferir el modo duplicación local**. Validar manualmente con cortes de conexión.
- La prueba automatizada valida cifrado y descifrado con archivos sintéticos, nunca credenciales o datos reales de clientes.

## Validación pendiente para producción

- CI Rust Windows y pruebas reales en Windows 10/11 con instalación/actualización.
- Prueba desde una PC distinta, usando la copia realmente **descargada** desde Drive, sin acceso a archivos de la primera computadora.
- Confirmar manualmente Drive sincronizado, desconexión de red, retoma de sincronización, espacio insuficiente, contraseña olvidada, cambio de carpeta, apagado durante copia y restauración completa.
- Valorar almacenamiento seguro de claves de uso automático con una opción específica del usuario, si no quiere desbloquear al abrir la aplicación (no introducir una clave persistida en claro).

Referencias: https://age-encryption.org/ y https://support.google.com/drive/answer/13401938?hl=es .
