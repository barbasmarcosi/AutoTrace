# AutoTrace Desktop v1 — prueba corta en Windows

**Solo cuando ingeniería confirme cinco CI SUCCESS del mismo commit.** No son ocho ensayos de desastre. Duración libre: importa que cada resultado se observe, no cumplir un plazo.

## Preparar
1. Abrir PR #36 y descargar el ZIP de la ejecución NSIS SUCCESS correspondiente al SHA exacto del candidato final.
2. Extraer el instalador .exe, autotrace-release-manifest.json y verify-acceptance-installer.ps1; ejecutar el verificador como indica ACCEPTANCE_PACKAGE_U0_U7.md, usando el commit completo y workflow run ID vistos directamente en GitHub.
3. Usar un perfil Windows de prueba y exclusivamente datos ficticios. No ignorar SmartScreen sin comprobar el origen y el SHA. El archivo está SIN FIRMA digital.

## Una única sesión de operación (U0 + U1 + U6)
- **U0:** instalar, abrir AutoTrace Desktop, comprobar que está vacío (sin clientes ficticios precargados), cerrar y volver a abrirlo. Registrar PASS/FAIL.
- **U1:** crear cliente «Cliente de prueba», auto patente «AF012BC», trabajo «Servicio de prueba» con un renglón de $12.500,25; cobrar $5.000,25 por transferencia, indicar una cuenta receptora de prueba; anular el cobro con motivo de al menos 12 caracteres. El saldo deberá volver a $12.500,25 y el recibo deberá seguir figurando como ANULADO. Exportar Excel completo (ocho hojas), cerrar y reabrir; comprobar que todo persiste. Registrar PASS/FAIL.
- **U6:** en ventanas normal y estrecha, usar teclado y mouse en formularios, menús, búsqueda y modal. Repetir con escalado Windows al 100%, 125% y 150% cuando sea posible; ningún botón indispensable debe quedar fuera de pantalla ni cerrarse el formulario al escribir. Registrar PASS/FAIL.
- **U7:** una vez exista una instalación anterior con **datos de prueba**, ejecutar el instalador nuevo, abrir y verificar que el trabajo y su historial permanecen intactos. La CI ya ensayó upgrade/rollback con un SQLite v1 sintético; U7 significa repetir el upgrade **en el equipo real**, no forzar un downgrade si no hace falta. Registrar PASS/FAIL/NO PROBADO.

## Devolver un resultado sin datos privados
Anotar versión de Windows, SHA del commit, run de Actions, U0/U1/U6/U7 PASS/FAIL/NO PROBADO y solo mensajes de error anonimizados. No adjuntar el Excel privado, SQLite, capturas con información real ni contraseñas de recuperación.

**Si U0, U1 o U6 falla, NO-GO: volver a ingeniería.** Si U7 no puede probarse porque no hay una versión anterior instalada, se deja NO PROBADO y se acuerda expresamente si la primera instalación v1 puede entregarse sin afirmar compatibilidad de upgrade físico.

**Fuera del bloqueo de la v1 offline:** U2–U4 recuperación en otra PC y U5 OAuth Google real. Las copias locales siguen disponibles, pero el proceso físico de recuperación no está certificado.
