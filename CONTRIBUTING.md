# Cómo contribuir

**Español** | [English](CONTRIBUTING.en.md)

¡Gracias por querer mejorar el Monitor del sistema! Toda ayuda cuenta: reportar un fallo, proponer una idea, mejorar la documentación o enviar código.

## Reportar un fallo o proponer una idea

Abre un [issue](https://github.com/soporte605/monitor_sistema/issues/new/choose) con la plantilla que corresponda:

- **Fallo:** incluye tu sistema operativo, la versión de la app (aparece junto al título), los pasos para reproducirlo y qué esperabas que pasara.
- **Idea:** cuenta qué problema resolvería y cómo te imaginas usarla.

Quita la información personal de las capturas antes de compartirlas (nombre del equipo, rutas con tu usuario…).

## Enviar cambios

El repositorio es público: cualquiera puede proponer cambios, pero todo entra mediante **Pull Request** y se revisa antes de fusionarse.

1. Haz un **fork** del repositorio y clónalo.
2. Crea una rama desde **`develop`** (no desde `main`):
   ```bash
   git checkout develop
   git checkout -b feature/mi-cambio   # o fix/…, docs/…, chore/…
   ```
3. Haz tus cambios y, antes de cada commit, ejecuta:
   ```bash
   cargo fmt                   # formatear
   cargo clippy --all-targets  # sin advertencias
   cargo test                  # tests
   ```
4. Sube la rama a tu fork y abre un Pull Request **hacia `develop`**. Si GitHub lo propone hacia `main`, cambia la rama de destino en el propio PR.

El **CI** comprueba cada Pull Request en macOS, Windows y Linux (formato, `clippy` y tests). Tiene que pasar para poder fusionarlo. La primera vez que contribuyes, el mantenedor tiene que aprobar que se ejecute.

## Convenciones

- **Idioma:** los textos de la interfaz y los comentarios del código van en **español**.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/) en español, con el título en imperativo: `feat: añadir…`, `fix: corregir…`, `docs: …`, `refactor: …`, `chore: …`, `test: …`.
- **CHANGELOG:** si el cambio se nota al usar la app, añade una línea en la sección `[Sin publicar]` de [CHANGELOG.md](CHANGELOG.md), bajo `Añadido`, `Cambiado`, `Corregido` o `Eliminado`.
- **Interfaz adaptable:** nada de anchos fijos salvo en columnas numéricas cortas; la ventana debe verse bien a cualquier tamaño.
- **Tests:** las funciones que no dependen de la interfaz llevan tests (`cargo test`).
- **Dependencias nuevas:** coméntalas antes en un issue.
- **Un cambio por Pull Request:** es más fácil de revisar y de deshacer si hace falta.

## Ramas

| Rama | Para qué |
|---|---|
| `main` | Solo versiones publicadas, cada una con su tag `vX.Y.Z`. Protegida. |
| `develop` | Integración del trabajo en curso. Protegida: todo entra por Pull Request. |
| `feature/…`, `fix/…`, `docs/…`, `chore/…` | Una rama por cambio, creada desde `develop`. |

## Licencia

Al contribuir, aceptas que tu aportación se publique bajo la [licencia MIT](LICENSE) del proyecto.
