# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto sigue [Versionado Semántico](https://semver.org/lang/es/).

## [Sin publicar]

## [0.2.2] - 2026-09-25

### Añadido
- README en inglés, con selector de idioma, resumen de características y acceso destacado a las descargas en ambas versiones.

### Corregido
- La barra de desplazamiento de la ventana completa ya no tapa el borde derecho del contenido (gráficas, barras y columna de memoria) al pasar el ratón: ahora ocupa su propio espacio.

## [0.2.1] - 2026-09-25

### Corregido
- El texto de las barras de núcleos, RAM y swap ahora se lee siempre: sobre la parte coloreada usa el color que más contrasta con ella (por ejemplo, texto oscuro sobre amarillo) y sobre el fondo mantiene el color normal.

## [0.2.0] - 2026-09-24

### Añadido
- Versiones descargables para macOS (universal), Windows y Linux en las Releases de GitHub, generadas automáticamente al publicar cada versión.
- Licencia MIT.
- Modo compacto: un icono de "imagen en imagen" en el encabezado (o ⌘⇧M / Ctrl+Shift+M) convierte la ventana en un widget pequeño y siempre visible con el uso de CPU y RAM y sus mini gráficas. Se mueve arrastrándolo y se vuelve a la ventana completa con doble clic, con el icono que aparece al pasar el ratón o con el mismo atajo.
- Referencias en las gráficas de CPU y memoria: escala de 0, 50 y 100 % a la izquierda y marcas de tiempo ("hace 60 s" / "ahora") debajo.

### Cambiado
- En Windows ya no se abre una ventana de consola junto a la app.
- Las cantidades de memoria se muestran en unidades binarias (MiB/GiB), que corresponden a las conversiones entre 1024.
- La lectura del sistema (CPU, memoria y procesos) se hace en un hilo en segundo plano; la interfaz solo dibuja y ya no se bloquea mientras se miden los datos.
- El listado de los 8 procesos principales se calcula una vez por lectura, en lugar de ordenar todos los procesos en cada redibujado.

### Corregido
- El relleno de las gráficas de CPU y memoria ya no se sale por encima de la línea cuando el valor sube o baja.
- La tabla de procesos ahora ocupa todo el ancho de la ventana; la columna "Nombre" se estira y recorta los nombres largos con "…" (el nombre completo aparece al pasar el ratón).
- Las barras de los núcleos se reparten en columnas según el ancho disponible en lugar de tener 180 px fijos.
- Las secciones de CPU, memoria y procesos ocupan siempre el ancho completo.

## [0.1.0] - 2026-09-24

### Añadido
- Ventana gráfica con eframe/egui.
- Uso de CPU global con gráfica en tiempo real (último minuto).
- Uso de CPU por núcleo con barras de color según la carga.
- Uso de memoria RAM y swap, con gráfica y barras.
- Tabla con los 8 procesos que más CPU consumen (PID, nombre, CPU y memoria).
- Encabezado con nombre del equipo, sistema operativo, tiempo encendido y versión de la app.

[Sin publicar]: https://github.com/soporte605/monitor_sistema/compare/v0.2.2...develop
[0.2.2]: https://github.com/soporte605/monitor_sistema/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/soporte605/monitor_sistema/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/soporte605/monitor_sistema/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/soporte605/monitor_sistema/releases/tag/v0.1.0
