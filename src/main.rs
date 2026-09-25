// Monitor del sistema en Rust
// GUI: eframe/egui  ·  Datos del sistema: sysinfo

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use sysinfo::{ProcessesToUpdate, System};

const HISTORIAL: usize = 120; // puntos en la gráfica (≈ 1 minuto a 500 ms)
const INTERVALO: Duration = Duration::from_millis(500);

struct Monitor {
    sys: System,
    ultima_lectura: Instant,
    hist_cpu: VecDeque<f32>,
    hist_ram: VecDeque<f32>,
}

impl Monitor {
    fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            sys,
            ultima_lectura: Instant::now() - INTERVALO,
            hist_cpu: VecDeque::with_capacity(HISTORIAL),
            hist_ram: VecDeque::with_capacity(HISTORIAL),
        }
    }

    /// Lee los datos del sistema si ya pasó el intervalo.
    fn actualizar(&mut self) {
        if self.ultima_lectura.elapsed() < INTERVALO {
            return;
        }
        self.ultima_lectura = Instant::now();

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        empujar(&mut self.hist_cpu, self.sys.global_cpu_usage());
        empujar(&mut self.hist_ram, porcentaje(self.sys.used_memory(), self.sys.total_memory()));
    }
}

fn empujar(hist: &mut VecDeque<f32>, valor: f32) {
    if hist.len() == HISTORIAL {
        hist.pop_front();
    }
    hist.push_back(valor);
}

fn porcentaje(usado: u64, total: u64) -> f32 {
    if total == 0 { 0.0 } else { usado as f32 / total as f32 * 100.0 }
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

/// Verde → amarillo → rojo según el porcentaje.
fn color_carga(p: f32) -> Color32 {
    match p {
        p if p < 50.0 => Color32::from_rgb(76, 175, 80),
        p if p < 80.0 => Color32::from_rgb(255, 193, 7),
        _ => Color32::from_rgb(244, 67, 54),
    }
}

/// Dibuja una gráfica de línea con relleno (0–100 %).
fn grafica(ui: &mut egui::Ui, datos: &VecDeque<f32>, color: Color32) {
    let (resp, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), 110.0), Sense::hover());
    let r = resp.rect;

    painter.rect_filled(r, 6.0, ui.visuals().extreme_bg_color);
    for i in 1..4 {
        let y = r.top() + r.height() * i as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
            Stroke::new(1.0, ui.visuals().faint_bg_color),
        );
    }

    if datos.len() < 2 {
        return;
    }
    let paso = r.width() / (HISTORIAL - 1) as f32;
    let inicio = r.right() - paso * (datos.len() - 1) as f32;
    let puntos: Vec<Pos2> = datos
        .iter()
        .enumerate()
        .map(|(i, v)| Pos2::new(inicio + paso * i as f32, r.bottom() - r.height() * v.clamp(0.0, 100.0) / 100.0))
        .collect();

    // Relleno: una franja vertical por segmento
    let relleno = color.gamma_multiply(0.18);
    for w in puntos.windows(2) {
        let rect = Rect::from_min_max(Pos2::new(w[0].x, w[0].y.min(w[1].y)), Pos2::new(w[1].x, r.bottom()));
        painter.rect_filled(rect, 0.0, relleno);
    }
    painter.add(egui::Shape::line(puntos, Stroke::new(2.0, color)));
}

/// Barra horizontal con texto encima.
fn barra(ui: &mut egui::Ui, p: f32, texto: String) {
    ui.add(
        egui::ProgressBar::new(p / 100.0)
            .text(texto)
            .fill(color_carga(p))
            .corner_radius(4.0),
    );
}

impl eframe::App for Monitor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.actualizar();
        let ctx = ui.ctx().clone();

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── Encabezado ─────────────────────────────
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Monitor del sistema").strong());
                    ui.label(RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let up = System::uptime();
                        ui.label(format!("Encendido: {}h {:02}m", up / 3600, (up % 3600) / 60));
                    });
                });
                ui.label(format!(
                    "{} · {} {}",
                    System::host_name().unwrap_or_default(),
                    System::name().unwrap_or_default(),
                    System::os_version().unwrap_or_default()
                ));
                ui.add_space(8.0);

                // ── CPU ────────────────────────────────────
                let cpu = self.sys.global_cpu_usage();
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong("CPU");
                        ui.label(RichText::new(format!("{cpu:.1} %")).color(color_carga(cpu)).strong());
                    });
                    grafica(ui, &self.hist_cpu, Color32::from_rgb(33, 150, 243));
                    ui.add_space(6.0);

                    ui.label(format!("{} núcleos", self.sys.cpus().len()));
                    egui::Grid::new("nucleos").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                        for (i, c) in self.sys.cpus().iter().enumerate() {
                            let p = c.cpu_usage();
                            ui.add_sized([180.0, 18.0], egui::ProgressBar::new(p / 100.0)
                                .text(format!("Núcleo {i}: {p:.0} %"))
                                .fill(color_carga(p))
                                .corner_radius(4.0));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                // ── Memoria ────────────────────────────────
                let (usada, total) = (self.sys.used_memory(), self.sys.total_memory());
                let p_ram = porcentaje(usada, total);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Memoria RAM");
                        ui.label(RichText::new(format!("{p_ram:.1} %")).color(color_carga(p_ram)).strong());
                    });
                    grafica(ui, &self.hist_ram, Color32::from_rgb(156, 39, 176));
                    ui.add_space(6.0);
                    barra(ui, p_ram, format!("{:.2} GB de {:.2} GB", gb(usada), gb(total)));

                    let (su, st) = (self.sys.used_swap(), self.sys.total_swap());
                    if st > 0 {
                        let p = porcentaje(su, st);
                        barra(ui, p, format!("Swap: {:.2} GB de {:.2} GB", gb(su), gb(st)));
                    }
                });
                ui.add_space(8.0);

                // ── Top procesos ───────────────────────────
                ui.group(|ui| {
                    ui.strong("Procesos que más CPU usan");
                    let mut procesos: Vec<_> = self.sys.processes().values().collect();
                    procesos.sort_by(|a, b| b.cpu_usage().total_cmp(&a.cpu_usage()));

                    egui::Grid::new("procesos").striped(true).num_columns(4).spacing([16.0, 4.0]).show(ui, |ui| {
                        ui.strong("PID");
                        ui.strong("Nombre");
                        ui.strong("CPU");
                        ui.strong("Memoria");
                        ui.end_row();
                        for p in procesos.iter().take(8) {
                            ui.label(p.pid().to_string());
                            ui.label(p.name().to_string_lossy());
                            ui.label(format!("{:.1} %", p.cpu_usage()));
                            ui.label(format!("{:.0} MB", p.memory() as f64 / 1024.0 / 1024.0));
                            ui.end_row();
                        }
                    });
                });
            });
        });

        // Pedir redibujado para que se actualice solo
        ctx.request_repaint_after(INTERVALO);
    }
}

fn main() -> eframe::Result {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Monitor del sistema")
            .with_inner_size([460.0, 760.0])
            .with_min_inner_size([380.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Monitor del sistema",
        opciones,
        Box::new(|_cc| Ok(Box::new(Monitor::new()))),
    )
}
