# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto sigue [Versionado Semántico](https://semver.org/lang/es/).

## [Sin publicar]

### Añadido
- Referencias en las gráficas de CPU y memoria: escala de 0, 50 y 100 % a la izquierda y marcas de tiempo ("hace 60 s" / "ahora") debajo.

### Cambiado
- Las cantidades de memoria se muestran en unidades binarias (MiB/GiB), que corresponden a las conversiones entre 1024.
- La lectura del sistema (CPU, memoria y procesos) se hace en un hilo en segundo plano; la interfaz solo dibuja y ya no se bloquea mientras se miden los datos.
- El listado de los 8 procesos principales se calcula una vez por lectura, en lugar de ordenar todos los procesos en cada redibujado.

### Corregido
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

[Sin publicar]: ../../compare/v0.1.0...develop
[0.1.0]: ../../releases/tag/v0.1.0
