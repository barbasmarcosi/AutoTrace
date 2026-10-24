# Compatibilidad de documentos Excel del taller — implementación incremental

Esta rama reconoce el diseño de **PRESUPUESTO - ESTIMACION REPARACION** y **REMITO TRABAJO** en una sola hoja, según los dos modelos compartidos (no se suben ejemplos con datos de clientes al repositorio).

## Lectura desde Excel existente

- Detecta B1: presupuesto o remito.
- Fecha J4, cliente I6, teléfono I7, dirección I8, correo I9, observaciones I10.
- Vehículo D12–D16: marca, modelo, patente, VIN y observación/kilometraje.
- Renglones B20:B45 (presupuesto) o B20:B44 (remito): descripción; H: cantidad; I: precio unitario; J: subtotal.
- Une las filas de descripción continuada sin cantidad ni precio con el renglón anterior, como el detalle largo de mano de obra del remito.
- Verifica total J47 (presupuesto) o J46 (remito). No confía en fórmulas para determinar los importes.
- Previsualiza antes de importar y exige seleccionar/confimar un vehículo existente; si la patente está informada debe coincidir. No inventa el cliente ni la patente faltante.
- Guarda huella SHA-256 de origen para impedir importar dos veces el mismo archivo; conserva tipo, número si existe y los renglones.
- Presupuesto no es una cuenta por cobrar: se excluye de ingresos, pagos pendientes y trabajos en ejecución.

## Escritura en el formato del taller

- Desde un trabajo **con renglones**, «Presupuesto Excel» o «Remito Excel» produce una hoja `Hoja1` con las mismas posiciones de título, fecha, cliente, vehículo, cantidades, precios y total.
- Incluye los renglones; no deduce costos internos a partir del precio cobrado.
- Permite configurar **un logo PNG/JPEG local**, que se incorpora al encabezado impreso con imagen; el archivo de imagen nunca se sube a GitHub.
- Para mantener compatibilidad con la plantilla fija, limita a 26 renglones y requiere que su suma coincida con el importe acordado; si no coincide, rechaza la exportación.
- Por ahora no se cargan automáticamente dirección/teléfono/correo propios del taller: esas preferencias aún deben modelarse; solo está disponible el nombre del taller. No es un clon pixel-perfect de estilos, bordes y formato.
- Si se desea conservar exactamente el Excel original con toda su diagramación, logo extraído y fórmulas, se requiere una fase adicional de uso de plantilla original.

## Respaldo independiente

El libro de **backup completo** de AutoTrace pasa a schema v2 e incorpora `DetalleTrabajos`; sigue leyendo libros v1. Los presupuestos y remitos comerciales NO son backups completos y no deben ingresar por «Restaurar respaldo Excel».

## QA

Tests usan documentos artificiales, sin PII. Se valida lectura y escritura de ambas clases de documento, saltos de línea, importes y correspondencia de vehículos. El instalador NSIS requiere CI verde y una validación manual Windows antes de entrega.
