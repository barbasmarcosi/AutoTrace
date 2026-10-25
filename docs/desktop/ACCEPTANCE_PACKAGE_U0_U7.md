# AutoTrace — Paquete de aceptación física U0–U7 (Windows)
**Estado:** paquete de ensayo, NO-GO para producción hasta U0–U7. **Sin firma digital**. Todo el material es para datos ficticios.

## 0. Procedencia exacta del paquete
1. Abrí [PR #36](https://github.com/barbasmarcosi/AutoTrace/pull/36). Anotá el SHA **completo de 40 caracteres** de la rama candidata; no uses SHA de un comentario anterior si la rama avanzó.
2. En el [workflow NSIS](https://github.com/barbasmarcosi/AutoTrace/actions/workflows/desktop-installer.yml), elegí una ejecución SUCCESS de **ese mismo SHA**. Descargá el artefacto `autotrace-desktop-windows-preview-unsigned`. No descargues ejecutables de foros ni instaladores de otro commit. Los artefactos de preview caducan a los siete días; si expiraron, corresponde reconstruir exactamente el SHA.
3. Descomprimí el ZIP. Debería incluir el instalador `AutoTrace_*_x64-setup.exe`, su `.sha256.txt`, `autotrace-release-manifest.json`, `verify-acceptance-installer.ps1`, `collect-physical-recovery-evidence.ps1` y este procedimiento. **Si faltan el JSON o el verificador, este paquete no sirve para emitir evidencia U0.**
4. En PowerShell de Windows, dentro de esa carpeta, ejecutá (reemplazando el SHA de ejemplo por el de GitHub):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\verify-acceptance-installer.ps1 -Installer .\AutoTrace_0.1.1_x64-setup.exe -Manifest .\autotrace-release-manifest.json -ExpectedCommit SHA_COMPLETO_DE_40_CARACTERES -EvidenceFile .\installer-preflight.json
```

El script no instala nada ni consulta Internet: exige nombre, tamaño, PE, SHA-256 exacto y que el commit coincida con uno obtenido de GitHub **por separado**. El JSON es una descripción del build, **NO** una firma digital/autenticidad frente a un atacante que pueda sustituir ZIP y JSON. Verificá la fuente y la advertencia de Windows/SmartScreen. Nunca omitas una alerta para ejecutar un binario de origen desconocido.

## U0 — instalar y arrancar en Windows A
Solo en equipo de prueba o perfil Windows dedicado, sin datos de clientes. Instalá el NSIS de preview **sin firma digital** que pasó el paso 0. Abrí, cerrá, reiniciá Windows y reabrí. Anotá en el registro U0 el build de Windows, versión de AutoTrace, resultado, fecha y SHA. Nunca informes PASS si no lo ejecutaste.

## U1 — operaciones, finanzas y Excel
Cargá 5 clientes ficticios, 7 autos y 12 trabajos (incluidas dos cuentas y pagos parciales). Usá una patente con cero, Unicode y cobros con decimales; cotejá importes, saldos, relación vehículo/cliente y Excel de ocho hojas. Cerrá, reabrí y comprobá persistencia. Nunca exportes datos reales al paquete de evidencia.

## U2 — backup USB físico independiente
Creá el kit de recuperación, **guardá la frase en papel separado** y exportá `.atbackup.age` a un pendrive descartable FAT/exFAT/NTFS de otro disco físico. Ejemplo:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\collect-physical-recovery-evidence.ps1 -PrimaryDataDirectory "$env:LOCALAPPDATA\ar.com.autotrace.desktop" -PortableArchive "E:\AutoTrace_prueba.atbackup.age" -EvidencePath .\usb-preflight.json
```

Este script comprueba disco físico diferente, `BusType=USB` y SHA-256 sin leer datos ni frase. **Solo acredita MEDIA_PREFLIGHT_ONLY**, no la recuperación. Guardá varias copias cifradas con distintos nombres; mantené los datos de prueba y nada más en el pendrive.

## U3 — recuperación de desastre en Windows B
PC **físicamente distinta** o instalación Windows limpia independiente, sin acceso a perfil, Google, disco ni claves de A y **sin Internet**. Llevá solamente el pendrive y la frase en papel por separado. Instalá exactamente el mismo EXE verificado, elegí “Probar copia cifrada portátil” y luego “Recuperar copia portátil”. Cotejá los IDs, contactos ficticios, autos, renglones, pagos, cuentas, saldos y Excel reconstruido. Registrar únicamente diferencias, conteos y resultados, no nombres ni capturas con datos.

## U4 — fallas físicas con información descartable
En un pendrive **dedicado a pruebas**, ensayá clave equivocada, archivo truncado y alteración de copia (trabajá sobre DUPLICADOS de la copia buena), dispositivo retirado durante una nueva escritura y falta de espacio. Ningún fallo debe decir “respaldo terminado” ni modificar la base activa. El último backup bueno debe restaurar B. **No hagas estas pruebas sobre un dispositivo que tenga datos importantes.**

## U5 — Drive real (bloqueado sin OAuth de AutoTrace)
Requiere una identidad OAuth Desktop legítima del proyecto y una **cuenta Google de prueba**. El conector de ChatGPT no la sustituye. Autorizá el permiso `drive.file`, subí exclusivamente `.atbackup.age`, interrumpí y reintentá, reiniciá para refresh, revocá y reconectá. Validá archivo remoto, MD5/tamaño y recuperación en Windows B sin acceso al equipo A. Si no hay Client ID, **U5 = BLOCKED**. No publiques Client ID junto a tokens, ni claves de recuperación, ni JSON de Google.

## U6 — WebView2 y operador
Operador no técnico completa alta, cobro, respaldo y recuperación. Comprobá teclado, diálogos, scroll, modo offline y escalas de DPI 100 %, 125 % y 150 %, después de reinicio.

## U7 — actualización y rollback
En un equipo de prueba y con backup válido, instalá una versión anterior compatible con datos sintéticos, actualizá al candidato exacto y revertí. Cotejá base, saldos y copias antes/después. La prueba GitHub de dos compilaciones no sustituye este ensayo.

## Registro y decisión
Usá `PHYSICAL_ACCEPTANCE_RECORD_TEMPLATE.json`; todos los gates parten de **PENDING**, U5 puede quedar **BLOCKED**, y solo vos podés consignar PASS tras observarlo. El preflight de instalador y de USB **nunca** cambian el estado U a PASS de forma automática. Guardá el registro en una carpeta privada; en GitHub solo un resumen anónimo sin nombres, rutas, series, contraseñas ni tokens.

**No mezclar los límites:** E0–E8 aprobó ingeniería automatizable; U0–U7 requiere hardware y decisiones humanas. Mientras U0–U7 no estén cerrados no fusionar PR #36, no publicar instalador definitivo ni llamarlo “producto 10/10 en producción”. Véase [contrato E/U](RELEASE_10_OF_10_ACCEPTANCE_MATRIX.md) y [issue #17](https://github.com/barbasmarcosi/AutoTrace/issues/17).
