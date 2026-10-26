# AutoTrace — presupuestos y remitos PDF (v1 offline)

**Estado:** función de exportación para el cliente; no es factura fiscal, respaldo de datos ni comprobante de pago.

## Desde la ficha de un trabajo

Con al menos un renglón de piezas o tareas, abrir **Documentos para el cliente**:
- **Presupuesto PDF:** descarga directamente el documento A4, marcado como estimación.
- **Remito PDF:** descarga el detalle A4 del trabajo.
- **Presupuesto Excel** y **Remito Excel:** permanecen como formatos alternativos, conservando la compatibilidad con las plantillas previas.
- **Resumen para cliente:** continúa ofreciendo impresión desde el navegador; puede emplearse *Guardar como PDF* para caracteres o símbolos Unicode que la tipografía básica del PDF directo no soporta.

Los documentos PDF incluyen nombre del taller, datos de contacto del cliente, marca/modelo/patente del vehículo, kilometraje, fecha, número de documento (si está informado), renglones, cantidades, precios unitarios y total. Se repiten las cabeceras y se numera la paginación en documentos largos.

**No se exportan** costos internos de piezas/mano de obra, margen, notas internas, medios/cuentas de cobro, información de otros clientes ni el archivo SQLite. Las cantidades y los importes se contrastan contra el estado validado; no se toleran discrepancias significativas entre el total del trabajo y sus renglones.

## Offline y experiencia Windows

La PWA descarga el archivo con el navegador, sin usar CDN ni servidor. AutoTrace Desktop muestra el cuadro nativo «Guardar presupuesto o remito PDF», exige extensión PDF y **no reemplaza** un archivo anterior automáticamente. La escritura usa un archivo temporal, sincronización y publicación final. Si hay un problema de formato, escritura o compatibilidad de caracteres, se muestra el error y no se marca la exportación como exitosa.

El PDF directo utiliza Helvetica y Helvetica-Bold de la tipografía estándar de PDF (WinAnsi). Los caracteres latinos utilizados habitualmente en Argentina (á, é, í, ó, ú, ñ y Ñ) están soportados. Un carácter que no puede representarse no se cambia de manera silenciosa. Para textos con otros alfabetos o símbolos usar **Resumen para cliente** y la impresión a PDF del sistema.

## Cobertura verificable

- Tests Node: estructura y xref PDF, campos y privacidad, precios con centavos, sanitización de cadenas, errores y paginación de 100 renglones.
- CI de navegador: descarga de ambos PDFs como archivos, PWA offline y Desktop con el comando nativo simulado.
- PDF.js independiente en CI: lectura del PDF, extracción de texto español, cantidad de páginas, importes, títulos y exclusión de información privada.
- Rust Windows: validación del archivo, rechazo de extensión/nombre incorrecto y preservación del PDF existente ante colisiones.
- Sigue siendo necesaria aceptación U0/U1/U6 en WebView2 real, especialmente con el diálogo nativo de guardado.

La exportación no cambia el formato de los respaldos completos ni los trabajos históricos.
