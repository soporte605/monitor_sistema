// Monitor del sistema en Rust
// GUI: eframe/egui  ·  Datos del sistema: sysinfo

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use sysinfo::{ProcessesToUpdate, System};

const HISTORIAL: usize = 120; // puntos en la gráfica (≈ 1 minuto a 500 ms)
const INTERVALO: Duration = Duration::from_millis(500);
const TOP_PROCESOS: usize = 8;

/// Datos de un proceso, ya listos para mostrar.
struct ProcesoInfo {
    pid: u32,
    nombre: String,
    cpu: f32,
    memoria: u64,
}

/// Una lectura completa del sistema, producida por el hilo lector.
struct Muestra {
    cpu_global: f32,
    nucleos: Vec<f32>,
    mem_usada: u64,
    mem_total: u64,
    swap_usada: u64,
    swap_total: u64,
    procesos: Vec<ProcesoInfo>,
}

/// Lanza un hilo que lee el sistema cada `INTERVALO` y envía una `Muestra` por el canal.
/// Así la interfaz nunca se bloquea esperando a `sysinfo`.
fn iniciar_lector(ctx: egui::Context) -> Receiver<Muestra> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut sys = System::new_all();
        loop {
            // El uso de CPU se calcula entre dos lecturas, por eso primero se espera
            thread::sleep(INTERVALO);
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_processes(ProcessesToUpdate::All, true);

            let mut procesos: Vec<ProcesoInfo> = sys
                .processes()
                .values()
                .map(|p| ProcesoInfo {
                    pid: p.pid().as_u32(),
                    nombre: p.name().to_string_lossy().into_owned(),
                    cpu: p.cpu_usage(),
                    memoria: p.memory(),
                })
                .collect();
            procesos.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            procesos.truncate(TOP_PROCESOS);

            let muestra = Muestra {
                cpu_global: sys.global_cpu_usage(),
                nucleos: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
                mem_usada: sys.used_memory(),
                mem_total: sys.total_memory(),
                swap_usada: sys.used_swap(),
                swap_total: sys.total_swap(),
                procesos,
            };

            // Si la ventana se cerró, el receptor ya no existe y el hilo termina
            if tx.send(muestra).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });

    rx
}

struct Monitor {
    rx: Receiver<Muestra>,
    actual: Option<Muestra>,
    hist_cpu: VecDeque<f32>,
    hist_ram: VecDeque<f32>,
}

impl Monitor {
    fn new(ctx: egui::Context) -> Self {
        Self {
            rx: iniciar_lector(ctx),
            actual: None,
            hist_cpu: VecDeque::with_capacity(HISTORIAL),
            hist_ram: VecDeque::with_capacity(HISTORIAL),
        }
    }

    /// Toma las muestras nuevas que haya enviado el hilo lector (sin bloquear).
    fn recibir(&mut self) {
        for m in self.rx.try_iter() {
            empujar(&mut self.hist_cpu, m.cpu_global);
            empujar(&mut self.hist_ram, porcentaje(m.mem_usada, m.mem_total));
            self.actual = Some(m);
        }
    }
}

fn empujar(hist: &mut VecDeque<f32>, valor: f32) {
    if hist.len() == HISTORIAL {
        hist.pop_front();
    }
    hist.push_back(valor);
}

fn porcentaje(usado: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        usado as f32 / total as f32 * 100.0
    }
}

const MIB: f64 = 1024.0 * 1024.0;
const GIB: f64 = 1024.0 * MIB;

/// Formatea bytes en unidades binarias (MiB o GiB), que es lo que dan las divisiones entre 1024.
fn formato_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else {
        format!("{:.0} MiB", b / MIB)
    }
}

/// Verde → amarillo → rojo según el porcentaje.
fn color_carga(p: f32) -> Color32 {
    match p {
        p if p < 50.0 => Color32::from_rgb(76, 175, 80),
        p if p < 80.0 => Color32::from_rgb(255, 193, 7),
        _ => Color32::from_rgb(244, 67, 54),
    }
}

/// Dibuja una gráfica de línea con relleno (0–100 %), con referencias
/// de porcentaje a la izquierda y de tiempo debajo.
fn grafica(ui: &mut egui::Ui, datos: &VecDeque<f32>, color: Color32) {
    const MARGEN_IZQ: f32 = 38.0; // espacio para "100 %"
    const MARGEN_INF: f32 = 14.0; // espacio para "hace 60 s … ahora"
    const ALTO_GRAFICA: f32 = 110.0;

    let (resp, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), ALTO_GRAFICA + MARGEN_INF),
        Sense::hover(),
    );
    let total = resp.rect;
    let r = Rect::from_min_max(
        Pos2::new(total.left() + MARGEN_IZQ, total.top()),
        Pos2::new(total.right(), total.top() + ALTO_GRAFICA),
    );

    let texto_tenue = ui.visuals().weak_text_color();
    let fuente = FontId::proportional(10.0);

    painter.rect_filled(r, 6.0, ui.visuals().extreme_bg_color);

    // Líneas guía cada 25 % y etiquetas en 0, 50 y 100 %
    for i in 0..=4 {
        let y = r.bottom() - r.height() * i as f32 / 4.0;
        if i > 0 && i < 4 {
            painter.line_segment(
                [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                Stroke::new(1.0, ui.visuals().faint_bg_color),
            );
        }
        if i % 2 == 0 {
            // Se aleja un poco de los bordes para que no se corte el texto
            let y_texto = y.clamp(r.top() + 6.0, r.bottom() - 6.0);
            painter.text(
                Pos2::new(r.left() - 6.0, y_texto),
                Align2::RIGHT_CENTER,
                format!("{} %", i * 25),
                fuente.clone(),
                texto_tenue,
            );
        }
    }

    // Referencias de tiempo
    let segundos = (HISTORIAL - 1) as f32 * INTERVALO.as_secs_f32();
    let y_tiempo = r.bottom() + MARGEN_INF / 2.0 + 1.0;
    painter.text(
        Pos2::new(r.left(), y_tiempo),
        Align2::LEFT_CENTER,
        format!("hace {segundos:.0} s"),
        fuente.clone(),
        texto_tenue,
    );
    painter.text(
        Pos2::new(r.right(), y_tiempo),
        Align2::RIGHT_CENTER,
        "ahora",
        fuente,
        texto_tenue,
    );

    if datos.len() < 2 {
        return;
    }
    let paso = r.width() / (HISTORIAL - 1) as f32;
    let inicio = r.right() - paso * (datos.len() - 1) as f32;
    let puntos: Vec<Pos2> = datos
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Pos2::new(
                inicio + paso * i as f32,
                r.bottom() - r.height() * v.clamp(0.0, 100.0) / 100.0,
            )
        })
        .collect();

    // Relleno: un trapecio por segmento, desde la línea hasta la base (siempre convexo)
    let relleno = color.gamma_multiply(0.18);
    for w in puntos.windows(2) {
        let trapecio = vec![
            w[0],
            w[1],
            Pos2::new(w[1].x, r.bottom()),
            Pos2::new(w[0].x, r.bottom()),
        ];
        painter.add(egui::Shape::convex_polygon(trapecio, relleno, Stroke::NONE));
    }
    painter.add(egui::Shape::line(puntos, Stroke::new(2.0, color)));
}

/// Recuadro de sección que ocupa todo el ancho disponible.
fn seccion(ui: &mut egui::Ui, contenido: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        contenido(ui);
    });
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
        self.recibir();

        egui::CentralPanel::default().show(ui, |ui| {
            // Hasta que llegue la primera lectura
            let Some(m) = &self.actual else {
                ui.centered_and_justified(|ui| ui.spinner());
                return;
            };

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // ── Encabezado ─────────────────────────────
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Monitor del sistema").strong());
                        ui.label(RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let up = System::uptime();
                            ui.label(format!(
                                "Encendido: {}h {:02}m",
                                up / 3600,
                                (up % 3600) / 60
                            ));
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
                    let cpu = m.cpu_global;
                    seccion(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.strong("CPU");
                            ui.label(
                                RichText::new(format!("{cpu:.1} %"))
                                    .color(color_carga(cpu))
                                    .strong(),
                            );
                        });
                        grafica(ui, &self.hist_cpu, Color32::from_rgb(33, 150, 243));
                        ui.add_space(6.0);

                        ui.label(format!("{} núcleos", m.nucleos.len()));
                        // Cuántas barras caben por fila según el ancho (mínimo 150 px cada una)
                        let separacion = 8.0;
                        let ancho = ui.available_width();
                        let caben = ((ancho + separacion) / (150.0 + separacion))
                            .floor()
                            .max(1.0) as usize;
                        let columnas = caben.min(m.nucleos.len().max(1));
                        let ancho_barra =
                            (ancho - separacion * (columnas - 1) as f32) / columnas as f32;

                        egui::Grid::new("nucleos")
                            .num_columns(columnas)
                            .spacing([separacion, 4.0])
                            .show(ui, |ui| {
                                for (i, &p) in m.nucleos.iter().enumerate() {
                                    ui.add_sized(
                                        [ancho_barra, 18.0],
                                        egui::ProgressBar::new(p / 100.0)
                                            .text(format!("Núcleo {i}: {p:.0} %"))
                                            .fill(color_carga(p))
                                            .corner_radius(4.0),
                                    );
                                    if (i + 1) % columnas == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                    ui.add_space(8.0);

                    // ── Memoria ────────────────────────────────
                    let p_ram = porcentaje(m.mem_usada, m.mem_total);
                    seccion(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.strong("Memoria RAM");
                            ui.label(
                                RichText::new(format!("{p_ram:.1} %"))
                                    .color(color_carga(p_ram))
                                    .strong(),
                            );
                        });
                        grafica(ui, &self.hist_ram, Color32::from_rgb(156, 39, 176));
                        ui.add_space(6.0);
                        barra(
                            ui,
                            p_ram,
                            format!(
                                "{} de {}",
                                formato_bytes(m.mem_usada),
                                formato_bytes(m.mem_total)
                            ),
                        );

                        if m.swap_total > 0 {
                            let p = porcentaje(m.swap_usada, m.swap_total);
                            barra(
                                ui,
                                p,
                                format!(
                                    "Swap: {} de {}",
                                    formato_bytes(m.swap_usada),
                                    formato_bytes(m.swap_total)
                                ),
                            );
                        }
                    });
                    ui.add_space(8.0);

                    // ── Top procesos ───────────────────────────
                    seccion(ui, |ui| {
                        ui.strong("Procesos que más CPU usan");
                        ui.add_space(4.0);

                        // La columna "Nombre" toma el espacio sobrante y recorta con "…"
                        let derecha = egui::Layout::right_to_left(egui::Align::Center);
                        TableBuilder::new(ui)
                            .striped(true)
                            .vscroll(false)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::exact(64.0))
                            .column(Column::remainder().clip(true))
                            .column(Column::exact(64.0))
                            .column(Column::exact(80.0))
                            .header(20.0, |mut fila| {
                                fila.col(|ui| {
                                    ui.strong("PID");
                                });
                                fila.col(|ui| {
                                    ui.strong("Nombre");
                                });
                                fila.col(|ui| {
                                    ui.with_layout(derecha, |ui| ui.strong("CPU"));
                                });
                                fila.col(|ui| {
                                    ui.with_layout(derecha, |ui| ui.strong("Memoria"));
                                });
                            })
                            .body(|mut cuerpo| {
                                for p in &m.procesos {
                                    cuerpo.row(20.0, |mut fila| {
                                        fila.col(|ui| {
                                            ui.label(p.pid.to_string());
                                        });
                                        fila.col(|ui| {
                                            ui.add(egui::Label::new(&p.nombre).truncate())
                                                .on_hover_text(&p.nombre);
                                        });
                                        fila.col(|ui| {
                                            ui.with_layout(derecha, |ui| {
                                                ui.label(format!("{:.1} %", p.cpu))
                                            });
                                        });
                                        fila.col(|ui| {
                                            ui.with_layout(derecha, |ui| {
                                                ui.label(formato_bytes(p.memoria))
                                            });
                                        });
                                    });
                                }
                            });
                    });
                });
        });
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
        Box::new(|cc| Ok(Box::new(Monitor::new(cc.egui_ctx.clone())))),
    )
}
