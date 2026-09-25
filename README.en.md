# Monitor del sistema — Rust system monitor

[Español](README.md) | **English**

A desktop resource monitor for **Windows, macOS and Linux**, built with Rust and egui. View CPU usage, memory and processes in real time, or keep a floating mini widget visible while you work.

**[Download the latest release](https://github.com/soporte605/monitor_sistema/releases/latest)** · [Usage](#usage) · [Report an issue](https://github.com/soporte605/monitor_sistema/issues)

## Features

- Overall and per-core CPU usage, with a chart covering the last minute.
- RAM and swap usage, with memory sizes in MiB/GiB.
- Top 8 processes by CPU usage, including PID and memory usage.
- An always-on-top compact widget with CPU and RAM mini charts.
- Downloads for macOS (Apple Silicon and Intel), Windows and Linux (x86_64).

The application interface is currently in Spanish.

<p align="center">
  <img src="captura.png" width="454" alt="System monitor showing CPU and RAM charts, per-core usage and processes">
</p>

## Download

Get the latest version from [Releases](https://github.com/soporte605/monitor_sistema/releases/latest). You do not need Rust to use a downloaded executable.

| Platform | File |
|---|---|
| macOS (Apple Silicon and Intel) | `monitor_sistema-vX.Y.Z-macos.zip` |
| Windows | `monitor_sistema-vX.Y.Z-windows.zip` |
| Linux (x86_64) | `monitor_sistema-vX.Y.Z-linux-x86_64.tar.gz` |

The app does not have an Apple or Microsoft developer signature, so your operating system may display a warning the first time you open it. Only proceed if you trust the download.

- **macOS:** extract the ZIP, move **Monitor del sistema** to Applications and open it. If macOS cannot verify the developer, go to **System Settings → Privacy & Security → Open Anyway**.
- **Windows:** extract the ZIP and run `monitor_sistema.exe`. If SmartScreen appears, choose **More info → Run anyway**.
- **Linux:** extract the archive with `tar -xzf monitor_sistema-*.tar.gz` and run `./monitor_sistema`.

## Usage

### Full window

The main window shows overall CPU usage and its one-minute history, per-core bars, RAM and swap usage, and the top 8 processes by CPU usage. Hover over a truncated process name to see its full name.

### Compact mode

A small, borderless, always-on-top widget shows CPU and RAM usage with mini charts.

<p align="center">
  <img src="captura-compacto.png" width="248" alt="Compact floating widget with CPU and RAM usage and mini charts">
</p>

| Action | How |
|---|---|
| Enter compact mode | Click the picture-in-picture icon at the top right, or press **⌘⇧M** on macOS / **Ctrl+Shift+M** on Windows and Linux |
| Move the widget | Drag it from any point |
| Restore the full window | Double-click, click the icon that appears on hover, or use the same shortcut |

The full window returns to its previous size and position. The widget remembers its position while the app remains open.

Some Linux window managers running Wayland may ignore the always-on-top setting.

## Build from source

Install [Rust](https://rustup.rs), then run:

```bash
git clone https://github.com/soporte605/monitor_sistema.git
cd monitor_sistema
cargo run --release
```

The app uses `eframe` / `egui` for the interface, `egui_extras` for tables and `sysinfo` for system data.

## Contributing

Bug reports and improvement suggestions are welcome in [Issues](https://github.com/soporte605/monitor_sistema/issues). For bugs, include your operating system, app version, steps to reproduce and expected behavior. Remove personal information from screenshots before sharing them.

Create branches from `develop` and submit pull requests to `develop`. Keep interface text and code comments in Spanish, and use Conventional Commits in Spanish. Before committing, run:

```bash
cargo fmt
cargo clippy --all-targets
cargo test
cargo build
```

Document user-visible changes in the `[Sin publicar]` section of [CHANGELOG.md](CHANGELOG.md). See the [Spanish workflow documentation](README.md#flujo-de-trabajo) for branch and release conventions.

## License

[MIT](LICENSE)
