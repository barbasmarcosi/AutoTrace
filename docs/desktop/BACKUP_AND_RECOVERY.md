# Respaldo automático, seguridad y recuperación — AutoTrace Desktop

**Estado:** diseño propuesto, no funcional hoy. **Objetivo:** protección del negocio aun sin servidor, con copias utilizables y restauración verificada.

## 1. Principios no negociables

- **Datos originales**: SQLite app-local es la única base primaria.
- **Doble respaldo**: backup consistente `.sqlite` y exportación `.xlsx` importable del **mismo snapshot lógico**, más manifiesto con checksums/versiones/conteos. La XLSX es una ruta de recuperación secundaria, no un sustituto de SQLite.
- **Fuera del equipo**: al menos una copia reciente en soporte separado y preferiblemente otra copia cifrada fuera del taller. «Backup en C:\\» es insuficiente ante robo/rotura/virus.
- **Nunca sobrescribir la última copia buena**; append/versiones, comprobación de escritura y retención prudente.
- **Privacidad:** backups completos incluyen nombres, teléfonos, patente, notas y costos. No publicarlos en GitHub, Pages, CI ni carpetas compartidas sin protección.

## 2. Cuándo se ejecuta y qué significa «automático»

**v1 sin servicio en segundo plano**:

- Al arrancar la aplicación, si no existe backup verificado del día, programar uno tras abrir base y validar integridad.
- Cada 4 horas **mientras AutoTrace está abierto** y luego de los cambios relevantes si corresponde por frecuencia, con debounce/cola única; no lanzar dos jobs simultáneos.
- Al cerrar (best effort) no sustituye el cron anterior: el SO puede terminar la app abruptamente.
- Si la PC duerme, se apaga o está sin almacenamiento externo, no hubo respaldo; al reabrir reintentar y mostrar alerta de atraso.
- La UI muestra fecha/hora de última copia **verificada**, siguiente ejecución condicionada a app abierta, destino y último error.

**v1.x opcional**: Task Scheduler de Windows configurado con consentimiento e instalador/herramienta auxiliar segura para copiar aun con app cerrada. Este job no se promete hasta demostrarlo instalado, con usuario bloqueado y equipo activo. Equipo apagado nunca realiza tareas.

## 3. Procedimiento de creación (transaccional/consistente)

1. Capturar ID de corrida, hora UTC y esquema; verificar espacio y carpeta autorizada, serializar tareas de backup.
2. Crear snapshot temporal por API SQLite Online Backup o mecanismo oficial equivalente; permitir escritura concurrente con consistencia. **Nunca copiar el archivo SQLite bruto mientras se usa.**
3. Verificar `PRAGMA integrity_check`, `foreign_key_check`, apertura y conteos del snapshot.
4. Exportar XLSX **desde el snapshot temporal**, no de otra lectura de la BD principal; incluir `Manifest` v1, checksum canónico y conteos. Probar parseo básico de XLSX generada.
5. Escribir `.tmp`, flush/sync según plataforma, cerrar handles y renombrar de modo seguro a nombres únicos. Escribir último un manifiesto de pareja finalizada que contenga hashes SHA-256 de artefactos y ubicaciones. Nunca marcar éxito si solo se creó uno de los dos.
6. Verificar checksum/reapertura de los archivos finales antes de actualizar `last_verified_backup_at`.
7. Mantener logs diagnósticos **sin datos privados** (ID, fecha, motivo de fallo, estado; no nombres/teléfonos/notas).
8. Retención por política, a posteriori y solo de backups previamente verificados, sin borrar la última versión saludable.

**Ubicación sugerida**: directorio app-specific con ACL/permiso usuario actual. Destino externo se elige con diálogo y se valida; no permitir acceso arbitrario a carpetas del sistema ni escribir rutas basadas en celdas Excel.

## 4. Retención y seguridad externa

Propuesta configurable inicial:

- 14 copias diarias + 8 semanales + 12 mensuales, ajustable por espacio libre.
- Usuario elige carpeta en **otro volumen/disco**, o sincronización opcional de paquetes **cifrados**.
- Disco externo desconectado tras copia semanal para limitar ransomware. No confundir «USB conectado todo el tiempo» con copia aislada.
- Google Drive/OneDrive **no son backend** ni garantía de restauración; carpeta sincronizada puede propagar borrados. Cifrar antes de enviar (`restic` como candidato, con repositorio y verificación probados, sin incrustar contraseña en código).
- Si la copia automática genera XLSX local legible, advertir que está **sin cifrar**. Copiar a la nube solo dentro de contenedor cifrado; los archivos Excel descargados manualmente nunca se cifran implícitamente.
- Guardar credenciales/frase de recuperación fuera de la PC y probar un recovery con esa clave. Una clave perdida vuelve inútiles los backups cifrados.
- Si no se configura destino externo, mostrar persistentemente «Solo copia en este equipo — sin protección ante pérdida del dispositivo».

**No se prometen 100% de disponibilidad, «cloud gratis para siempre» ni cero gastos de hardware.** Cero costo mensual obligatorio solo aplica si usa almacenamiento local y servicios existentes.

## 5. Restauración: procedimientos seguros

### Fallo lógico (se borró algo por error)

1. Ver historial, fecha, volumen y conteos de cada respaldo; permitir inspección/preview sin reemplazar.
2. Exportar una copia de seguridad del estado actual (pre-restore), aun cuando el usuario considere que está roto.
3. Restaurar a una base **temporal** de prueba, validar integridad/relaciones/versiones/filas/cantidades; informar diferencias.
4. Confirmación clara «REEMPLAZAR» y swap en ruta controlada, sin escrituras concurrentes; si falla, conservar el original intacto.
5. Reiniciar conexión SQLite y verificar conteos, identificadores, saldos y configuración; registrar evento de restauración sin detalles de clientes.

### PC rota, robada o reemplazada

1. Instalar AutoTrace Desktop del mismo/correspondiente schema.
2. Recuperar backup SQLite de volumen externo o archivo cifrado con su clave, **sin la PC vieja**.
3. Verificar SHA-256/manifest y consistencia y restaurar. Si SQLite no es recuperable, importar el XLSX completo validado.
4. Comprobar presencialmente 3 clientes, 3 vehículos, últimas 5 órdenes y valores económicos con el usuario.
5. Configurar nuevo destino y hacer una copia inicial verificada en la nueva computadora.

### Fallos obligatorios para QA

- Disco lleno durante XLSX / SQLite / manifiesto.
- Extracción USB a mitad de guardado.
- Carpeta externa inaccesible y reconexión posterior.
- Daño/cambio de bytes de XLSX y SQLite; checksum no coincide.
- App finalizada abruptamente durante rename/restauración.
- Importe 0, centavos, campos Unicode, celdas maliciosas/fórmulas Excel.
- Retención agresiva con solo un backup válido: no borrar la última copia.
- Nube sin sincronizar aunque exista archivo local.
- Tras restaurar un backup viejo, se preserva una copia de la versión más nueva que había antes.

## 6. Indicadores de salud visibles

- Estado: **Protegido** solo cuando hay último snapshot verificado y al menos una copia externa confirmada según política. **Solo local** / **Atrasado** / **Error** en casos restantes.
- Hora de última copia completa, destino y tamaño.
- Diferenciar «se generó archivo», «se verificó» y «se copió fuera de la PC».
- Alertas al superar 24 h sin backup verificado y al superar 7 días sin copia externa; configurables, sin notificaciones falsas si PC estuvo apagada.
- Acción «Probar restauración» en entorno temporal: validación sin reemplazar el trabajo actual.
- Simulacro de recuperación desde soporte externo al menos mensual para la primera instalación y tras cambios del motor de backup.

## 7. Gate de producción

AutoTrace Desktop **NO ESTÁ LISTO** hasta que:
- Se haya restaurado un backup en un entorno limpio.
- La prueba de consistencia SQLite e import XLSX de forma independiente pasen.
- Se haya verificado un soporte distinto del disco principal.
- Exista un procedimiento escrito que el cliente pueda ejecutar sin el desarrollador.
- Los datos locales previos se hayan migrado con un JSON de respaldo conservado.
