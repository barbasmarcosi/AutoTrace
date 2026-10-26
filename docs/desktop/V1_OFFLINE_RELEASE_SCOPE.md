# AutoTrace Desktop v1 — alcance offline y aceptación reducida

**Estado: listo para probar, no certificado físicamente.** Una PC Windows, SQLite local, sin cuenta Google obligatoria ni servidor.

## Funcionalidades incluidas
- Clientes, vehículos, trabajos, presupuestos, renglones, costos, cobros parciales, cuentas receptoras, anulaciones motivadas e historial.
- Exportación/restauración Excel privado v4, compatibilidad con respaldos v1–v3, reportes por cliente sin costos internos, importación de documentos antiguos, CSV/JSON según edición.
- Datos offline SQLite con control de revisión; sin sembrado de información demostrativa en Desktop.
- Copias automáticas locales, copias externas y cifrado portátil disponibles **sin garantía de recuperación física probada**. Guardar copias solamente en la PC no protege ante robo o avería del disco.
- Instalador Windows NSIS de prueba, sin firma digital. SHA-256, manifiesto, CI de SQLite/Excel/Chromium, instalación y upgrade/rollback sintético.

## Contrato reducido aprobado por el propietario
| Gate | Condición para edición v1 utilizable |
|---|---|
| E0–E8 | Cinco CI verdes del mismo SHA, sin P0/P1 pendientes |
| U0 | Instalar, abrir y reiniciar en una PC Windows real |
| U1 | Operar cliente, auto, trabajo, cobro, anulación, Excel, cierre y reapertura |
| U6 | Verificar WebView2, modales, teclado, foco y DPI en Windows real |
| U7 | Actualizar una instalación real preservando los datos; downgrade físico puede permanecer no probado y declarado |

**U2–U4 (USB/recuperación física/fallos físicos) y U5 (Google Drive OAuth real) quedan fuera del bloqueo del primer release.** No están aprobados, aunque la funcionalidad siga disponible. La exclusión NO permite omitir los tests automáticos de pérdida/corrupción de datos.

**Decisión:** hasta que U0/U1/U6/U7 pasen con observación real, marcar el instalador como preview, mantener el PR Draft y no fusionar ni publicar producción. La firma digital no está disponible; SmartScreen puede mostrar un aviso.

Usar exclusivamente datos ficticios en el procedimiento corto [V1_QUICK_WINDOWS_CHECK.md](./V1_QUICK_WINDOWS_CHECK.md). Descargar el paquete del workflow NSIS SUCCESS del SHA final y verificar EXE, archivo SHA y manifiesto con el verificador incluido. La evidencia debe omitir identidades de clientes, contraseñas y rutas privadas.

AutoTrace no es facturación fiscal ni un sistema bancario: anular un cobro no reintegra el dinero por medios electrónicos. U2–U5 permanecen en el contrato completo [RELEASE_10_OF_10_ACCEPTANCE_MATRIX.md](./RELEASE_10_OF_10_ACCEPTANCE_MATRIX.md).
