# Plan maestro — AutoTrace Desktop local-first

**Revisión:** 2026-10-21 · **Estado:** APPROVED-FOR-IMPLEMENTATION PROPOSAL / no ejecutado · **Rama:** `plan/desktop-sqlite-excel-backups-20261021`

## 1. Objetivo y definición de éxito

Transformar la PWA actual, pensada para uso local mediante IndexedDB, en una aplicación de escritorio para **una persona y una computadora**. Almacenar clientes, vehículos y trabajos de manera durable, conservar el diseño ya trabajado, operar desconectado y recuperarse de una rotura o reemplazo del equipo. **Costo mensual obligatorio de infraestructura: US$ 0** (sin servicios web contratados). Almacenamiento fuera del equipo/disco externo puede necesitar un dispositivo o una cuenta existente.

El éxito no es «se generó un XLSX»; es poder **restaurar en una instalación limpia, sin red y con IDs, saldos, relaciones, configuración e historial consistentes** y demostrarlo con una prueba automatizada y una restauración manual.

### Definición de entregable

- Instalador para Windows 10/11 x64 (NSIS recomendado); binarios firmados solo si se contrata/dispone de certificado; no afirmar que el ejecutable está firmado de otro modo.
- Un solo origen de datos local en SQLite, con migraciones versionadas, constraints e índices.
- Uso del mismo frontend de AutoTrace, sin grandes rediseños; navegadores/Pages conservan el modo demo IndexedDB.
- Copias automáticas SQLite + XLSX cuando la aplicación está en funcionamiento, ambas desde un snapshot coherente, con estado visible y política de versiones.
- Exportación manual privada del XLSX completo, importación/recuperación, y un XLSX de entrega por cliente que no incluya columnas internas.
- Recuperación comprobada desde una copia externa después de simular pérdida total de datos locales.

## 2. Situación real del código (baseline)

- `site/app.js`: UI y operaciones, guardado en IndexedDB.
- `site/model.mjs`: validación de estructura `clients[]`, `vehicles[]`, `orders[]` y `settings`.
- `site/search.mjs`, `site/pickers.mjs`, `site/styles.css`: lógica reutilizable.
- Los clientes y vehículos usan IDs de aplicación; `plate` se normaliza y es única; un vehículo pertenece a un cliente; una orden pertenece a un vehículo.
- Campos económicos de orden: `partsCost`, `laborCost`, `charged` y `paid` (acumulado, **no** existe tabla histórica con fechas/medios de pagos). Los estados actuales: `received`, `progress`, `waiting`, `done`, `delivered`.
- Los JSON antiguos constituyen material migrable; no borrar la PWA ni cambiar `site/model.mjs` en una migración destructiva.

## 3. Arquitectura objetivo y aislamiento de componentes

```text
                Interfaz AutoTrace (actual)
                  /                 \
       IWebDataGateway         DesktopDataGateway
       (IndexedDB PWA)            (comandos Tauri)
           |                          |
      Pages / demo                  Rust (validación de entradas,
                                    transacciones, filesystem acotado)
                                       |
                                SQLite + migraciones
                                       |
                             BackupCoordinator
                               /           \
                      snapshot .sqlite     export .xlsx
                               \           /
                               manifiesto/estado/sha256
                                       |
                           carpeta externa configurada
                          (copias cifradas opcionales)
```

El frontend no debe escribir directamente archivos arbitrarios ni recibir permisos Tauri globales `fs:allow-*`. Exponer comandos *limitados*: CRUD validado, exportación, copia y restauración; parametrizar SQL; usar rutas de datos de aplicación y diálogos nativos para destinos elegidos por el usuario. Considerar plugin SQL de Tauri solo para conexiones estrictamente permitidas; preferir comandos Rust para flujos de respaldo/transacciones/restore.

**Persistencia:** SQLite con `PRAGMA foreign_keys=ON`, claves/índices para ID y patente normalizada, `schema_migrations`, versión de esquema, `CHECK` económicos no negativos, `paid <= charged`, restricciones de estados y fechas. Guardar dinero en centavos enteros o decimal exacto con conversión segura (nunca confiar en binary floating point para cálculos de importes). Mantener de cara a UI los campos y IDs compatibles con las estructuras existentes.

**Entidad opcional futura:** `payments` cuando se solicite fecha/medio por cada pago. En v1 mantener `paid` acumulado; migrar a ledger solo con regla explícita para datos históricos sin inventar fecha.

**Convivencia PWA:** el frontend invoca una interfaz `DataGateway` que tiene implementación IndexedDB o Desktop. Nunca ejecutar desktop SQL bridge desde el sitio Pages. La importación a escritorio es explícita y reversible; en ningún caso se borran datos de origen al importar.

## 4. Fases de implementación — entregables y gates

### Fase D0: contrato y caracterización, 2–4 h

- Congelar baseline en un commit/fixture con casos límite: tildes, patentes con espacios, caracteres Unicode, fechas, valores 0 y decimales, estado entregado, clientes sin autos, autos con varios trabajos, demo/real.
- Precisar el sistema operativo del cliente (Windows es hipótesis priorizada), espacio disponible y destino del respaldo fuera del disco principal.
- Definir `DataGateway`, contrato de serialización y diferencias de confianza entre UI/PWA/desktop.
- Gate: fixture JSON actual pasa ida-vuelta de importación a esquema destino sin pérdida.

### Fase D1: Tauri + SQLite, 5–8 h

- Crear `desktop/` con Tauri v2, lockfiles y configuración de permisos mínimos.
- Definir migración SQLite v1 para `clients`, `vehicles`, `orders`, `settings`, metadatos de backups y `schema_migrations`.
- Implementar lectura, alta, edición, baja segura y transacciones. Mantener el comportamiento actual de búsquedas/paginación.
- Adaptador para la UI actual. Prohibir destructivos directos de la UI en los archivos del sistema.
- Gate: CRUD, FK, unicidad, montos y reinicio reales; PWA Pages sin regresiones.

### Fase D2: migración desde IndexedDB/JSON, 2–4 h

- Crear exportación JSON de la PWA, compatible con `validateData` existente.
- Flujo desktop «Importar registros actuales»: análisis y preview, comprobación de duplicados de IDs/patentes, preview conteos y conflictos, copia de seguridad anterior, aplicación **atómica** o rechazo completo.
- Mostrar «datos de ejemplo» diferenciados; no mezclar demo con reales silenciosamente.
- Gate: todos los IDs/relaciones/fechas/importes/settings equivalentes, segunda importación no duplica, rollback íntegro ante fallo; JSON de origen permanece intacto.

### Fase D3: exportación e importación Excel v1, 5–8 h

- Implementar contrato en [EXCEL_FORMAT_V1.md](EXCEL_FORMAT_V1.md), evaluando librería XLSX mantenida, licencia y comportamiento real en Tauri, con versión fijada.
- `Exportar todo`: libro completo con IDs y relaciones, validaciones y `Manifest`; posibilidad de exportar manualmente a destino escogido.
- `Importar Excel`: dos modos explícitos: restaurar respaldo verificado o importar archivo editado con revisión estricta; preview antes de mutar.
- `Exportar para cliente`: informe filtrado, legible, sin datos privados/financieros internos.
- Gate: XLSX de salida → instalación vacía → XLSX importado → equivalencia semántica de datos, con cero huérfanos y cero importaciones parciales; pruebas de ataques en celdas/fórmulas.

### Fase D4: respaldos automáticos y recuperación, 5–9 h

- Orquestar snapshot SQLite mediante SQLite Online Backup API (o técnica oficial equivalente; no copiar `.db` en caliente).
- Producir Excel **desde el mismo snapshot** que el `.sqlite`, registrar manifiesto/checksum, finalizar escrituras mediante archivo temporal + flush + rename. El respaldo se considera exitoso solo al verificar lectura/consistencia.
- Plan local inicial: a la primera apertura del día y luego cada 4 horas de uso; siempre después de operaciones críticas si se superó la ventana, con debounce. No afirmar que se ejecuta con equipo apagado; **evaluar Task Scheduler opt-in** para backups con app cerrada en Windows en una fase adicional.
- Mantener retención por antigüedad/espacio, jamás purgar la última copia válida ni limpiar copias de terceros.
- Restaurar SQLite o Excel a base temporal, validar, respaldo pre-restore y swap seguro con retorno ante fallos.
- Gate: restauración desde unidad externa tras eliminar base local, sin red y sin pérdida de referencialidad.

### Fase D5: interfaz de respaldo, operación y entregable, 3–5 h

- Ajustes > Datos y copias: ubicación local, destino externo, frecuencia, «Última copia verificada», «Próxima copia prevista» (solo con app en marcha), espacio libre, alertas, botón «Crear copia ahora», «Exportar Excel», «Importar Excel», «Restaurar».
- Flujo de confirmación explícito para reemplazar datos (nombre del taller, cantidad de registros, hora y diferencias); ningún botón decorativo.
- Error claro cuando USB/carpeta/nube no están disponibles, sin declarar éxito falso.
- Instalador Windows por workflow de build de rama/release separado del Pages workflow; smoke instalación/upgrade/desinstalación sin borrar datos por defecto.
- Gate: criterios [ACCEPTANCE_MATRIX.md](ACCEPTANCE_MATRIX.md) completos, prueba manual Windows/Safari no aplica al desktop WebView2; no mergear cambios funcionales sin pasar.

### Fase D6 (opcional y NO bloqueante): backup externo cifrado

- Configurar destino físico distinto (USB) o carpeta sincronizada existente con Google Drive/OneDrive **cifrada antes de subir**; nunca asumir que sincronización significa restauración probada.
- Evaluar restic para snapshots cifrados, con verificación y retención; contraseña protegida y mecanismo de recuperación probado.
- Gate: equipo de prueba limpio recupera datos con copia y contraseña disponible fuera de la PC.

## 5. Coste/alcance y estimación

Sin servidor recurrente ni pagos de hosting. Estimación orientativa de **22–38 horas asistidas por IA** para D0–D5, con posible variación por toolchain Rust/Windows y problemas de migración. No prometer 12 horas ni considerar la versión productiva terminada por compilación verde. La generación de instaladores Windows podría necesitar runners y límites de Actions, firma de código es adicional/opcional; disco externo también.

**No agregar ProtocolIA como dependencia de ejecución.** Para este proyecto de escritorio y tres entidades, el riesgo de adaptar un generador de aplicaciones servidor supera el beneficio. Podemos modelar contratos de dominio de manera independiente y usar ProtocolIA como experimento no bloqueante.

## 6. Riesgos, decisiones y límites

- **La XLSX no es sustituto íntegro de la copia SQLite.** La biblioteca de Excel puede reformatear cifras, fechas y tipos; respaldar ambas formas y comprobar igualdad lógica.
- **No asumir backup = archivo en C:\\ mismo disco.** Debe existir destino externo y recuperación comprobada.
- **No guardar secretos en GitHub ni escribir Excel real en el repositorio.** Usar datos sintéticos en pruebas públicas.
- **No incluir imágenes/adjuntos en el XLSX** si se incorporan luego; se necesitará paquete adjunto/manifest actualizado.
- **Las copias automáticas contienen datos personales.** Permisos restrictivos, cifrado remoto y previsualizaciones cuidadosas.
- **Excel puede truncar identificadores o normalizar patentes** si no se fuerzan celdas texto; testear a conciencia.
- **Si se pierde la contraseña de cifrado**, una copia cifrada puede ser irrecuperable. Preparar kit de recuperación offline.
- **UI no introduce extras funcionales hasta cerrar resiliencia.** Prioridad: fiabilidad y recuperación sobre cosmética.

## 7. Política de merges y despliegue

- Esta rama/PR es **solo planificación**, mantener en **Draft**.
- Implementar en PRs pequeños con gates por fase; revisar HEAD de `main` antes de cada integración.
- `.github/workflows/pages.yml` sigue sirviendo a la demo pública y no debe subir ejecutables ni datos.
- Crear CI desktop separada con binarios solo de prueba/instalador, permisos mínimos y sin secretos embebidos.
- Nunca publicar respaldos, bases SQLite con clientes, contraseñas ni planillas reales como artefactos de Actions.

## 8. Fuentes

- https://v2.tauri.app/plugin/sql/
- https://v2.tauri.app/distribute/windows-installer/
- https://www.sqlite.org/backup.html
- https://v2.tauri.app/plugin/file-system/
