# Monitor del sistema (Rust + egui)

Muestra en tiempo real: uso de CPU (gráfica + por núcleo), memoria RAM/swap y los procesos que más CPU consumen.

![Captura](captura.png)

## Ejecutar
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
