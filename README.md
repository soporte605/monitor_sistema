# Monitor del sistema (Rust + egui)

Muestra en tiempo real: uso de CPU (gráfica + por núcleo), memoria RAM/swap y los procesos que más CPU consumen.

![Captura](captura.png)

## Descargar

Descarga la última versión desde [Releases](https://github.com/soporte605/monitor_sistema/releases/latest):

| Sistema | Archivo |
|---|---|
| macOS (Apple Silicon e Intel) | `monitor_sistema-vX.Y.Z-macos.zip` |
| Windows | `monitor_sistema-vX.Y.Z-windows.zip` |
| Linux (x86_64) | `monitor_sistema-vX.Y.Z-linux-x86_64.tar.gz` |

La app no está firmada por Apple ni por Microsoft, así que la primera vez el sistema avisará:

- **macOS:** descomprime el `.zip`, mueve **Monitor del sistema** a Aplicaciones y ábrelo. Si macOS dice que no puede comprobar el desarrollador, ve a **Ajustes del Sistema → Privacidad y seguridad** y pulsa **Abrir igualmente**. También puedes quitar la marca de cuarentena desde la terminal:
  `xattr -dr com.apple.quarantine "/Applications/Monitor del sistema.app"`
- **Windows:** descomprime el `.zip` y ejecuta `monitor_sistema.exe`. Si aparece SmartScreen, pulsa **Más información → Ejecutar de todas formas**.
- **Linux:** descomprime con `tar -xzf monitor_sistema-*.tar.gz` y ejecuta `./monitor_sistema`.

## Compilar desde el código
1. Instala Rust: https://rustup.rs
2. En esta carpeta: `cargo run --release`

Dependencias: `eframe` (ventana y GUI) y `sysinfo` (datos del sistema). Funciona en Windows, macOS y Linux.

## Flujo de trabajo

### Ramas
- `main`: solo versiones publicadas. Cada versión lleva un tag `vX.Y.Z`.
- `develop`: integración del trabajo en curso.
- `feature/<nombre>`: una rama por funcionalidad, creada desde `develop` y fusionada de vuelta con un Pull Request.

### Versionado semántico
La versión vive en `Cargo.toml` (`version = "X.Y.Z"`) y se muestra en la app.
- **MAJOR**: cambios incompatibles.
- **MINOR**: funcionalidades nuevas compatibles.
- **PATCH**: correcciones.

Mientras la versión sea `0.x`, el proyecto está en desarrollo inicial.

### Commits
Se usa [Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/): `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`…

### Publicar una versión
1. Mover lo de `[Sin publicar]` en `CHANGELOG.md` a una nueva sección con la versión y fecha.
2. Actualizar `version` en `Cargo.toml` y ejecutar `cargo build` (actualiza `Cargo.lock`).
3. Commit `chore(release): vX.Y.Z`, fusionar `develop` en `main` y crear el tag:
   `git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin main --tags`

Al subir el tag, GitHub Actions (`.github/workflows/release.yml`) compila la app para macOS, Windows y Linux y crea la Release con los archivos descargables y las notas de esa versión del CHANGELOG.

## Licencia

[MIT](LICENSE)
