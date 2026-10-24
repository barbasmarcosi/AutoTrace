# D8-B — Recuperación portátil sin Google Cloud

**Implementación en revisión:** [PR #12](https://github.com/barbasmarcosi/AutoTrace/pull/12) encadenado a los PR #11 y #10. **No equivale a una recuperación ensayada en una PC física.**

## Qué protege este mecanismo

AutoTrace Desktop puede crear una copia privada en formato `.atbackup.age` para usar un pendrive, disco externo o una ubicación segura que el operador elija. El contenido está cifrado mediante age con una frase de recuperación portable. Dentro de cada archivo cifrado se verifican SQLite, Excel y un manifiesto SHA-256.

**No hace falta Google Cloud, OAuth, instalar Google Drive para escritorio ni tener conexión.** Los archivos SQLite y Excel originales continúan en el almacenamiento local habitual, por lo que no sustituyen a protección de Windows o cifrado del disco.

## Primera configuración recomendada

1. Abrir **Configuración y respaldos → Copias de seguridad · Google Drive**. Aunque Google no esté configurado, aparecen las opciones **Crear kit de recuperación**, **Exportar copia cifrada a USB**, **Probar copia cifrada portátil** y **Recuperar copia portátil**.
2. Pulsar **Crear kit de recuperación** únicamente si nunca se configuró una frase para copias anteriores. AutoTrace genera una frase aleatoria fuerte de 68 caracteres con dos UUIDv4 independientes (244 bits de aleatoriedad efectiva).
3. **Transcribir o imprimir la frase y guardarla fuera de la PC**, separada del dispositivo con respaldos. El diálogo solicita los últimos ocho caracteres y una confirmación explícita de resguardo. La clave no se escribe en GitHub ni se envía a Google.
4. Al confirmar, el almacén seguro del usuario Windows guarda la frase únicamente para usarla automáticamente en esa computadora. Esto no sustituye la frase guardada por separado; si se destruye Windows o se pierde el usuario, se necesitará esa frase manualmente.
5. Pulsar **Exportar copia cifrada a USB**. Ingresar y confirmar **la misma frase** del kit, elegir una ruta en el pendrive o disco y guardar. No elegir un nombre que sobrescriba un archivo existente.
6. Abrir **Probar copia cifrada portátil**, elegir el archivo recién exportado e ingresar su frase. La prueba restaura por separado el SQLite y el Excel en bases temporales y compara contenido **sin reemplazar los registros actuales**.
7. Retirar el dispositivo de manera segura. Conservá **varias versiones** en medios seguros y mantené el kit fuera de la computadora original.

## Recuperación en una computadora completamente nueva

1. Instalar una versión de AutoTrace Desktop compatible con el respaldo.
2. Conectar el medio con el `.atbackup.age`. La cuenta de Google y el usuario de Windows anteriores **no son necesarios**.
3. Pulsar **Probar copia cifrada portátil** y verificar que clientes, vehículos y trabajos se hayan reconstruido correctamente desde el archivo.
4. Pulsar **Recuperar copia portátil**, seleccionar el mismo archivo e ingresar la frase del kit. AutoTrace muestra los datos que se van a reemplazar y solicita confirmación. Si existen registros previos, intenta generar una copia local antes de modificar la base.
5. Verificar cuentas, cobros, órdenes y vehículos; generar un respaldo cifrado nuevo y resguardar el medio.

## Protección de la información y límites actuales

- La copia portátil incluye **información privada cifrada** de clientes, vehículos, trabajos, cuentas de cobros, finanzas y preferencias. Nunca usar el Excel sin cifrar para traslados inseguros.
- La frase de recuperación no puede reconstruirse si se pierde. No hay clave maestra universal ni servicio propio de AutoTrace que pueda recuperarla.
- Crear un kit **no equivale a haber generado un respaldo**: después hay que exportar y **probar** el archivo.
- El método de copia portátil se ejecuta **cuando el operador lo solicita**; no programa escrituras periódicas en un USB desconectado. Las copias locales automáticas y los respaldos directos de Drive son funciones separadas.
- Para restablecer respaldos age anteriores hay que conservar **la frase usada en cada período**. Nunca cambiar silenciosamente la frase de un taller con historiales que deban conservarse.
- Si el USB se desconecta, se queda sin espacio o da error de integridad, el archivo no se considera respaldo terminado y la base principal permanece intacta.
- La exportación deja un snapshot local SQLite+Excel verificado en la carpeta privada de backups, genera un archivo age en staging privado y solo al final promociona la copia cifrada externa. Los temporales cifrados se borran después de la operación correcta o fallida; la limpieza tras un cierre abrupto requiere controles adicionales.
- El archivo cifrado portable **no debe subirse a ubicaciones inseguras junto a la frase de recuperación**. Para protección ante robo de PC, mantener un medio distinto y la frase separada.

## Aceptación pendiente

- CI del SHA final en Windows: tests Rust + Node y empaquetado NSIS. 
- Windows físico: almacenamiento USB con sistema FAT32/exFAT/NTFS, dispositivo retirado, disco sin espacio, instalación fresca.
- Ensayo de recuperación desde USB conectado a **otra computadora**, sin acceso a datos de la primera.
- Una política de recuperación de errores al desconectar durante escritura y limpieza de temporales tras cierres imprevistos.

**Google Cloud no bloquea ninguno de estos ensayos offline.**
