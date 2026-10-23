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
5. Las cuentas se administran desde Ajustes. No se eliminan en esta etapa, porque están referenciadas por el historial. Solo deben guardarse nombre, tipo y alias opcional, nunca contraseñas ni tokens.

## Datos históricos
- Los valores `paid` anteriores se conservan incluso si no existían pagos con medio/fecha/cuenta. La diferencia `paid - sum(payments.amount)` se muestra como **histórico sin clasificar**, sin inventar un destino.
- Para trabajos posteriores, el campo `paid` no se edita directamente desde el formulario de trabajo: se usa **Registrar cobro**.
- En caso de correcciones del historial ya contabilizado, documentar y definir un flujo de anulación con auditoría en una fase siguiente antes de habilitar borrar cobros.

## Respaldos Excel
- **v3 (completo y privado)** agrega hojas `Cuentas` y `Cobros`, y un SHA-256 canónico sobre todos los datos; el lector admite respaldos v1, v2 y v3.
- Los documentos presupuesto/remito y el **informe Excel para cliente** no contienen cuentas receptoras, referencias bancarias, costos internos ni datos financieros de otros clientes.
- No usar este formato como extracto bancario ni prueba contable/fiscal.
- Validar recuperación cruzada PWA ↔ Desktop y restauración desde otra PC antes de distribuir a producción.

## Pendientes de aceptación
- Smoke real de UI con efectivo y transferencia a cuenta nueva y existente, pagos parciales, reingreso, factura de mano de obra y sin renglones, sin contaminación de reportes del cliente.
- CI Node + Rust Windows sobre el HEAD.
- Prueba física con Excel real y recuperación en instalación limpia, con copia fuera del disco principal.
