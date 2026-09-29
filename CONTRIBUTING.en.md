# Contributing

[Español](CONTRIBUTING.md) | **English**

Thanks for helping improve Monitor del sistema! Every contribution counts: bug reports, ideas, documentation and code.

## Report a bug or suggest an idea

Open an [issue](https://github.com/soporte605/monitor_sistema/issues/new/choose) using the matching template:

- **Bug:** include your operating system, the app version (shown next to the title), steps to reproduce and the expected behavior.
- **Idea:** describe the problem it would solve and how you would use it.

Remove personal information from screenshots before sharing them, such as your computer name or paths containing your username.

## Submit changes

The repository is public: anyone can propose changes, but everything goes through a **pull request** and is reviewed before merging.

1. **Fork** the repository and clone your fork.
2. Create a branch from **`develop`**, not from `main`:
   ```bash
   git checkout develop
   git checkout -b feature/my-change   # or fix/…, docs/…, chore/…
   ```
3. Make your changes. Before each commit, run:
   ```bash
   cargo fmt                   # format
   cargo clippy --all-targets  # no warnings
   cargo test                  # tests
   ```
4. Push the branch to your fork and open a pull request **against `develop`**. If GitHub suggests `main`, change the base branch in the pull request.

**CI** checks every pull request on macOS, Windows and Linux (formatting, `clippy` and tests), and it must pass before merging. For first-time contributors, the maintainer has to approve the CI run.

## Architecture

Before writing code, read [ARCHITECTURE.md](ARCHITECTURE.md) (in Spanish, with an English summary at the top). It explains how the project is organized, where it is heading and how to add a new feature.

## Conventions

- **Language:** interface text and code comments are written in **Spanish**. Issues and pull requests can be written in English or Spanish.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) in Spanish, with an imperative title: `feat: añadir…`, `fix: corregir…`, `docs: …`, `refactor: …`, `chore: …`, `test: …`.
- **CHANGELOG:** if users will notice the change, add a line to the `[Sin publicar]` section of [CHANGELOG.md](CHANGELOG.md), under `Añadido` (added), `Cambiado` (changed), `Corregido` (fixed) or `Eliminado` (removed).
- **Responsive interface:** avoid fixed widths except for short numeric columns; the window must work at any size.
- **Tests:** logic that does not depend on the interface needs tests (`cargo test`).
- **New dependencies:** discuss them in an issue first.
- **One change per pull request:** it is easier to review and to revert if needed.

## Branches

| Branch | Purpose |
|---|---|
| `main` | Published releases only, each tagged `vX.Y.Z`. Protected. |
| `develop` | Integration of ongoing work. Protected: every change goes through a pull request. |
| `feature/…`, `fix/…`, `docs/…`, `chore/…` | One branch per change, created from `develop`. |

## License

By contributing, you agree that your contribution will be published under the project's [MIT license](LICENSE).
