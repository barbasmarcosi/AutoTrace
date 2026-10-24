# Contrato Excel XLSX importable — AutoTrace v1

**Estado:** propuesta de formato (no implementado). **Principio:** representar de modo fiel el modelo actual, con claves estables, sin interpretar Excel como motor de cálculo ni ejecutar contenido.

## 1. Productos de exportación separados

1. **Copia automática XLSX completa**: archivo en carpeta privada configurada, con metadatos y todos los registros; se genera a partir del mismo snapshot que el backup SQLite. Restaurable.
2. **Exportar todo a Excel** (manual): idéntico esquema/columnas, usuario elige carpeta, advertencia «Contiene información privada y costos internos». Importable posteriormente.
3. **Excel para entregar al cliente** (manual): *otro libro y otro contrato*: uno o varios trabajos del propietario seleccionado, con datos de vehículo y concepto, importes convenidos/saldo solo si se solicita; **sin** `partsCost`, `laborCost`, margen, notas internas de cliente/vehículo/trabajo, otros clientes, IDs internos o datos del taller ajenos al documento. Nunca ofrecerlo como opción de restauración.
4. **Importar Excel / restaurar copia**: reconocer la firma de libro y versión, validar relaciones y advertir antes de sustituir; no aceptar libros destinados al cliente como respaldos.

Un archivo `.xlsx` normal **no está cifrado**. Guardar copias automáticas legibles en carpeta restringida; para destinos externos remotos usar cifrado antes de sincronizar. El XLSX exportado manualmente es responsabilidad del usuario en su destino.

## 2. Libro de respaldo formal

**Nombre sugerido:** `AutoTrace_Backup_YYYY-MM-DD_HH-mm-ss_<short-id>.xlsx` en UTC para colisiones y orden. Encabezados fijos en fila 1; ningún dato real ni fórmulas en la documentación o fixtures.

| Hoja | Columnas exactas v1 (en orden) |
| --- | --- |
| `Manifest` | `clave`, `valor` |
| `Clientes` | `id`, `nombre`, `telefono`, `email`, `notas`, `creado_el` |
| `Vehiculos` | `id`, `cliente_id`, `patente`, `marca`, `modelo`, `anio`, `kilometraje`, `vin`, `color`, `proximo_servicio`, `notas`, `creado_el` |
| `Trabajos` | `id`, `vehiculo_id`, `titulo`, `categoria`, `estado`, `fecha_apertura`, `fecha_cierre`, `kilometraje`, `descripcion`, `costo_repuestos`, `costo_mano_obra`, `importe_acordado`, `total_pagado`, `notas`, `creado_el` |
| `Configuracion` | `nombre_taller`, `moneda`, `datos_demostracion` |

### Manifest (pares clave / valor)

Requeridos: `format=autotrace-backup-xlsx`, `schema_version=1`, `export_kind=full`, `created_at_utc` en ISO8601, `snapshot_id` UUID, `app_version`, `source_schema_version`, `timezone=America/Argentina/Buenos_Aires` para presentación, `count_clients`, `count_vehicles`, `count_orders`, `canonical_sha256` (hash de una proyección canónica de datos parseados, no del zip XLSX con metadatos cambiantes). La versión de canonización y orden se documenta y fija en tests; checksum se verifica tras leer el libro.

**No modificar nombres de hojas/columnas sin subir `schema_version`.** Si una futura versión agrega `Pagos`, definirá nueva versión con semántica de conciliación y migración explícita; el `total_pagado` actual es acumulado, sin fechas históricas.

## 3. Semántica de celdas y tipos

- IDs, FKs, patente, teléfono, VIN y cadenas con ceros iniciales: **texto**, jamás números/formato General. Todas las claves preservan exactamente su valor original.
- Fechas `YYYY-MM-DD` como texto ISO o vacío; no fechas Excel con timezone implícita. `creado_el` conserva el formato validado en el modelo actual.
- `anio` y `kilometraje` numéricos finitos no negativos (año entero dentro del rango admitido por la app); importes numéricos de precisión centavos, sin símbolos `$` en los datos.
- La moneda es la configuración (`ARS`, `USD`, `EUR`), no una conversión automática.
- `estado` solo: `received`, `progress`, `waiting`, `done`, `delivered`; durante visualización ofrecer etiquetas en castellano, persistir claves canónicas.
- Cadenas vacías y campos opcionales: política estable; no confundir vacío con cero ni inventar fechas. Fórmulas/celdas hipervínculo no son valores válidos en el libro importado.
- Identificadores que empiecen `=`, `+`, `-` o `@` y otras entradas peligrosas deben exportarse **como celdas de tipo cadena no fórmula**; verificar al reabrir la planilla. No «sanear» destruyendo contenido legítimo o IDs.
- Valores monetarios: redondeo explícito half-up/centavos según decisión de dominio, con prueba 0.01/0.1/0.3 y negativos rechazados. Sumatorias y margen se recalculan desde datos validados en app, no fórmulas guardadas en planilla.

## 4. Validación y modos de importación

### Modo A: restaurar respaldo generado por AutoTrace

1. Seleccionar `.xlsx` formal, validar tamaño comprimido/descomprimido y cantidad de filas.
2. Comprobar `Manifest` y versión soportada; rechazar firma alterada, fórmulas, macros, referencias externas, ZIP malicioso, archivos que simulen XLSX.
3. Leer hojas en un **modelo temporal**; tipar/normalizar según schema; validar IDs únicos, patentes normalizadas únicas, FKs `Vehiculos.cliente_id` y `Trabajos.vehiculo_id`, montos y `total_pagado <= importe_acordado`, fechas, enumeraciones, conteos, checksum.
4. Mostrar resumen de datos actuales vs importados, fecha/origen, cantidad y registros conflictivos.
5. Crear copia de seguridad del estado actual y probar que se puede leer; requerir confirmación explícita para reemplazo.
6. Restaurar mediante **transacción** o base temporal + swap atómico, con `PRAGMA foreign_key_check` e `integrity_check` antes de activarla. Si algo falla, mantener estado actual y registrar motivo; nunca dejar media importación.

### Modo B: importar Excel editado manualmente

- No llamar a esto «respaldo verificado» si las filas ya no coinciden con `canonical_sha256`; habilitar **«Importar planilla editada»** con advertencia de que sus datos fueron alterados.
- Requiere mismas hojas, IDs y validaciones, ofrece preview/errores por fila-columna; política inicial: **reemplazo completo**, no mezcla silenciosa.
- Si el usuario desea actualizar uno o varios registros, definir en versión posterior estrategia explícita por claves y reporte de conflictos; no implementarla implícitamente como upsert.
- Igual respaldo previo y transacción; rechazar libros `export_kind=client_report` y formatos antiguos no migrados.

### Salvaguardas

- Límites del modelo inicial como referencia: clientes <= 15.000, vehículos <= 30.000, trabajos <= 100.000; también límite de bytes, filas vacías masivas, profundidad y ratio de expansión ZIP.
- No importar desde `.xlsm`, Excel con macros, archivos falsos ni ejecutar fórmulas. No confiar en títulos de hoja ni nombres de archivo como evidencia de autenticidad.
- Los campos desconocidos son error hasta tener una política de evolución documentada; preview distingue avisos y errores bloqueantes.
- Seguridad ante overwrite, falta de espacio, permisos, Excel abierto/bloqueado, libro cortado a mitad y fallos de escritura.

## 5. Paridad de ida/vuelta

Prueba obligatoria (fixture con datos sintéticos): importar JSON v1 → SQLite A → exportar Excel A → restaurar en SQLite B → comparar representación canónica A == B sin depender de orden de filas, formatos XLSX o espacios en patentes. Ejercitar dos exportaciones consecutivas, IDs Unicode, vacíos, 0, `paid=charged`, patentes con ceros, importes de centavos, borrados y demo flag. No mutar A durante el ensayo.

La restauración exacta desde `.sqlite` sigue siendo primera elección ante desastre; la recuperación desde Excel debe funcionar como segunda vía y ser probada de forma independiente.

## 6. Librería y dependencias

Comparar ExcelJS y SheetJS CE en spike técnico, fijar versiones, revisar licencia y reporte de vulnerabilidades, probar en Tauri/WebView2 con generación y lectura real. **No seleccionar biblioteca por nombre sin haber comprobado roundtrip y seguridad.**

## 7. Producto: botones y textos

- Ajustes > Resguardos: **Crear respaldo ahora**, **Exportar Excel completo**, **Importar/Restaurar Excel**, **Último respaldo correcto**.
- Clientes > Ficha: **Exportar informe para cliente** (selector de fechas y servicios).
- Antes de exportar todo: «Este Excel contiene datos personales, costos y notas internas; no compartir con terceros».
- Antes de restaurar: «Se reemplazarán N clientes, N vehículos y N trabajos. Guardaremos una copia del estado actual». Sin opción de sobrescribir sin aceptación consciente.
