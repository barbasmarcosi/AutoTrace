# AutoTrace XLSX Lab en GitHub Pages

Se sirve la versión estable en /AutoTrace/ desde main y la versión en desarrollo en /AutoTrace/lab/ desde la rama del PR #1.

El builder verifica que la PWA estable sea idéntica byte a byte al sitio main, y modifica exclusivamente el artefacto LAB: IndexedDB autotrace-lab-v1, service worker y CacheStorage con prefijo autotrace-lab-shell-, manifest separado y advertencia visible. No hay datos de clientes en la publicación: los archivos XLSX se procesan localmente en el navegador.

Importante: ambos sitios comparten origen github.io, por lo que esta separación protege frente a mezclas accidentales de datos, **no** establece un límite de seguridad entre sitios de origen diferente.

Comprobación: abrir /AutoTrace/lab/, ir a Ajustes, importar Presupuesto o Remito Excel, revisar detalle y exportarlo; descargar un respaldo Excel completo, importar el respaldo y comprobar que la PWA estable en /AutoTrace/ conserva sus datos. El Excel del cliente excluye datos internos.

Limitaciones: las exportaciones web conservan estructura y datos de la plantilla, pero no los logos/estilos originales pixel-perfect; no hay backups programados fuera del navegador ni sincronización entre equipos. No usar datos reales del cliente antes de las verificaciones.
