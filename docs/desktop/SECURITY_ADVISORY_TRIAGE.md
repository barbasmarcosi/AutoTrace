# AutoTrace — RustSec: revisión de 7 advertencias no bloqueantes

**Estado:** investigación pendiente de confirmar reachability Windows con el job `Classify advisory warnings in Windows dependency tree`. No representa una revisión de seguridad completa ni excluye riesgos futuros.

## Evidencia

- En el commit `a291af842efdac0d5cf6e394f5c032672a9208f2`, [RustSec](https://github.com/barbasmarcosi/AutoTrace/actions/runs/40152257449) encontró **0 vulnerabilidades** en el `Cargo.lock` de 606 paquetes, **7 advertencias** (6 bibliotecas sin mantenimiento, 1 sonido de implementación).
- Las advertencias **no se desactivaron** y la auditoría sigue siendo un gate. La nueva CI Windows enumera explícitamente las bibliotecas alcanzables por dependencias normal/build del target `x86_64-pc-windows-msvc`.
- Esta clasificación sirve para decidir si actualizar un padre o si el código afectado solo entra por una plataforma no distribuida. El hecho de que un paquete figure en el lockfile NO demuestra que se compile para Windows.

| Advisory | Paquete | Cadena de dependencias rastreada en Cargo.lock | Interpretación provisional y acción |
|---|---|---|---|
| RUSTSEC-2026-0374 | `basic-toml` | `autotrace-desktop → age → i18n-embed-fl → i18n-config` | Sin mantenimiento; revisar caminos de carga de configuración. No es una vulnerabilidad reportada. |
| RUSTSEC-2026-0173 | `proc-macro-error2` | `autotrace-desktop → age → i18n-embed-fl → proc-macro-error2` | Dependencia de macros; revisar mantenimiento/alternativa upstream. |
| RUSTSEC-2024-0370 | `proc-macro-error` | `autotrace-desktop → tauri → gtk → gtk3-macros → proc-macro-error` | Dependencia GTK; verificar exclusión del target Windows. |
| RUSTSEC-2026-0371 | `zbus_names` | `autotrace-desktop → rfd → ashpd → zbus → zbus_names` | Integración portal Linux; comprobar target. |
| RUSTSEC-2026-0370 | `zvariant` | `autotrace-desktop → rfd → ashpd → zbus → zvariant` | Integración portal Linux; comprobar target. |
| RUSTSEC-2026-0372 | `zvariant_derive` | `autotrace-desktop → rfd → ashpd → zbus → zvariant → zvariant_derive` | Derivado portal Linux; comprobar target. |
| RUSTSEC-2024-0429 | `glib` | `autotrace-desktop → tauri → gtk → glib` | Sonoridad de `glib::VariantStrIter`; revisar exclusión del Windows y no asumir ausencia en Linux. |

## Criterio de cierre

1. El Windows-target resolution debe finalizar sin error y publicar `WINDOWS_TARGET_PRESENT/ABSENT` para las siete dependencias.
2. Si `glib` está en el grafo Windows, examinar con más profundidad si el iterador vulnerable es alcanzable; no basta la ausencia de llamadas directas en AutoTrace.
3. Los paquetes sin mantenimiento siguen como deuda documentada. Se revisan versiones de padres y advisories antes de cada release; si pasan a vulnerabilidad o afectan datos, **NO-GO**.
4. La prueba `cargo audit` completa sigue sin filtros, suppressions ni ignores.
5. Esta revisión no sustituye las pruebas U0–U7 de hardware/Drive real.

Documentar el resultado de la CI del SHA candidato definitivo antes de marcar **E5 PASS**.
