# Finanzas y cobros trazables — AutoTrace LAB / Desktop

## Qué representa cada indicador
- **Importe acordado:** valor de los trabajos cargados, aunque todavía no se hayan cobrado.
- **Pagos recibidos:** suma del campo `paid` de cada trabajo no presupuestario.
- **Saldo por cobrar:** `max(charged - paid, 0)` por trabajo, excluyendo presupuestos.
- **Resultado estimado:** importe acordado menos costo directo de repuestos y costo interno de mano de obra. No representa caja ni utilidad neta contable; no incluye alquiler, impuestos, comisiones, gastos generales ni otros costos indirectos.
- **Resultado de mano de obra:** suma de renglones `items.kind === 'labor'` (cantidad × precio), menos `laborCost`, **solo** para órdenes que tienen renglones explícitos de mano de obra. Las órdenes históricas sin desglose se excluyen de esta métrica en vez de inventar su facturación laboral. Los presupuestos se excluyen de toda métrica real y de cuentas a cobrar.

## Cobros
1. Crear trabajo con importe acordado y costos; el monto recibido se registra después desde **Registrar cobro**.
2. Elegir importe, fecha y medio: efectivo, transferencia, débito, crédito, QR/billetera u otro.
3. Para transferencia, tarjeta o QR elegir la **cuenta propia receptora** (banco o billetera) o crear una nueva sin abandonar el flujo. Se puede adjuntar referencia/comprobante como texto no sensible.
4. Los cobros se agregan individualmente a `order.payments[]` y actualizan `order.paid` sin permitir superar el monto acordado. El historial se muestra en la ficha y Finanzas suma por cuenta.
5. El identificador de cada cobro es **único en todo el taller**, no solo dentro de su trabajo. La huella SHA-256 de un documento de origen tampoco puede asociarse a dos trabajos. La PWA y el motor SQLite rechazan duplicados antes de persistir para impedir contabilizaciones dobles; los registros históricos sin huella siguen admitidos.
5. Las cuentas se administran desde Ajustes. No se eliminan en esta etapa, porque están referenciadas por el historial. Solo deben guardarse nombre, tipo y alias opcional, nunca contraseñas ni tokens.

## Datos históricos
- Los valores `paid` anteriores se conservan incluso si no existían pagos con medio/fecha/cuenta. La diferencia `paid - sum(payments.amount)` se muestra como **histórico sin clasificar**, sin inventar un destino.
- Para trabajos posteriores, el campo `paid` no se edita directamente desde el formulario de trabajo: se usa **Registrar cobro**.
- **Anulación auditable:** desde la ficha del trabajo se marca un cobro con `voidedAt` y `voidReason` (motivo de 12 a 500 caracteres), sin borrar el asiento original. `paid` y la suma por cuentas excluyen el cobro anulado; el historial muestra importe, medio, fecha original, fecha de anulación y motivo. Una anulación contable NO ejecuta un reintegro bancario real.
- Los trabajos con cobros históricos, incluso anulados, no se pueden borrar desde la interfaz.
- El Excel completo pasa a **v4** con dos columnas nuevas de anulación en `Cobros`. Se mantienen lectores v1/v2/v3 para backups anteriores y SHA-256 canónico. La base SQLite conserva esquema v1 porque los campos nuevos son opcionales.
- **Límite:** una importación completa de respaldo o la manipulación fuera de AutoTrace del archivo SQLite son operaciones privilegiadas. Este flujo todavía no equivale a un libro mayor legalmente inmutable.

## Respaldos Excel
- **v3 (completo y privado)** agrega hojas `Cuentas` y `Cobros`, y un SHA-256 canónico sobre todos los datos; el lector admite respaldos v1, v2 y v3.
- Los documentos presupuesto/remito y el **informe Excel para cliente** no contienen cuentas receptoras, referencias bancarias, costos internos ni datos financieros de otros clientes.
- No usar este formato como extracto bancario ni prueba contable/fiscal.
- Validar recuperación cruzada PWA ↔ Desktop y restauración desde otra PC antes de distribuir a producción.

## Criterios de aceptación automatizados
- Browser Chrome real (GitHub Actions): importación de remito sin cliente/vehículo (creación en el mismo flujo), reutilización de cliente/vehículo existentes, transferencia a cuenta nueva, QR a cuenta existente, cobro final en efectivo, doble envío accidental, saldo final cero y visualización de Finanzas.
- Node: cálculo de mano de obra cuando hay desglose (sin inventarlo en trabajos anteriores), histórico sin clasificar, cuentas y cobros referenciales, respaldo privado XLSX v3 con varias cuentas/medios, lectura idéntica y rechazo de un cambio malicioso de cuenta destino.
- El formulario impide persistir dos veces un cobro por múltiples clics mientras se guarda; el botón Guardar queda inhabilitado temporalmente.
- Las cuentas de Ajustes son **etiquetas locales**: no existe integración bancaria.

## Pendientes reales antes de producción
- Verificar los workflows Chrome y Windows del commit definitivo del PR.
- Prueba manual de UX con Firefox/Safari móvil, teclado, fecha y flujo de registro; conciliación visual de informes Excel con archivos del taller.
- Recuperación real en una instalación limpia de Windows y ensayo de copia fuera del disco principal. Las pruebas automatizadas no sustituyen este gate.
- Futura mejora: libro mayor append-only protegido contra edición directa de SQLite, conciliación de reintegros y firma de evidencia; la anulación v4 registra un motivo y no transfiere dinero.
- Cualquier importe denominado «neto» sigue siendo **estimado** (antes de impuestos, gastos fijos y comisiones), no saldo bancario.
