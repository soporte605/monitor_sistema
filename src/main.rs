// Monitor del sistema en Rust
// GUI: eframe/egui  ·  Datos del sistema: sysinfo

// En Windows, que la versión release no abra una consola junto a la ventana
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    Vec2, ViewportCommand, WindowLevel,
};
use egui_extras::{Column, TableBuilder};
use sysinfo::{ProcessesToUpdate, System};

const HISTORIAL: usize = 120; // puntos en la gráfica (≈ 1 minuto a 500 ms)
const INTERVALO: Duration = Duration::from_millis(500);
const TOP_PROCESOS: usize = 8;

const TAM_VENTANA_MIN: Vec2 = Vec2::new(380.0, 400.0);
const TAM_WIDGET: Vec2 = Vec2::new(240.0, 70.0);
const MARGEN_WIDGET: f32 = 16.0; // separación del borde derecho de la pantalla
const MARGEN_SUPERIOR_WIDGET: f32 = 48.0; // deja libre la barra de menús de macOS
const COLOR_CPU: Color32 = Color32::from_rgb(33, 150, 243);
const COLOR_RAM: Color32 = Color32::from_rgb(156, 39, 176);
/// ⌘⇧M en macOS, Ctrl+Shift+M en Windows y Linux (⌘M ya es "minimizar" en macOS).
const ATAJO_COMPACTO: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::M);

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
    /// Si la ventana está en modo compacto (widget flotante).
    compacto: bool,
    /// Posición y tamaño de la ventana completa, para restaurarla al salir del modo compacto.
    ventana_completa: Option<(Pos2, Vec2)>,
    /// Última posición del widget, para que vuelva donde se dejó.
    pos_widget: Option<Pos2>,
}

impl Monitor {
    fn new(ctx: egui::Context) -> Self {
        Self {
            rx: iniciar_lector(ctx),
            actual: None,
            hist_cpu: VecDeque::with_capacity(HISTORIAL),
            hist_ram: VecDeque::with_capacity(HISTORIAL),
            compacto: false,
            ventana_completa: None,
            pos_widget: None,
        }
    }

    /// Cambia entre la ventana completa y el widget compacto siempre visible.
    fn alternar_modo(&mut self, ctx: &egui::Context) {
        let (exterior, interior, monitor) = ctx.input(|i| {
            let v = i.viewport();
            (v.outer_rect, v.inner_rect, v.monitor_size)
        });

        if self.compacto {
            self.pos_widget = exterior.map(|r| r.min);
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::Normal));
            ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(TAM_VENTANA_MIN));
            if let Some((pos, tam)) = self.ventana_completa {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(tam));
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
            }
        } else {
            self.ventana_completa = exterior.zip(interior).map(|(e, i)| (e.min, i.size()));
            let pos = self
                .pos_widget
                .unwrap_or_else(|| pos_inicial_widget(monitor, TAM_WIDGET));
            // El mínimo se baja antes de encoger, si no el sistema no deja reducir la ventana
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(TAM_WIDGET));
            ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
            ctx.send_viewport_cmd(ViewportCommand::Resizable(false));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(TAM_WIDGET));
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
        }
        self.compacto = !self.compacto;
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

/// Dónde aparece el widget la primera vez: esquina superior derecha de la pantalla.
fn pos_inicial_widget(monitor: Option<Vec2>, tam: Vec2) -> Pos2 {
    let x = match monitor {
        Some(m) => (m.x - tam.x - MARGEN_WIDGET).max(0.0),
        None => MARGEN_WIDGET,
    };
    Pos2::new(x, MARGEN_SUPERIOR_WIDGET)
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

    dibujar_serie(&painter, r, datos, color, 2.0);
}

/// Dibuja la línea del historial con su relleno dentro de `r` (0 % abajo, 100 % arriba).
fn dibujar_serie(
    painter: &egui::Painter,
    r: Rect,
    datos: &VecDeque<f32>,
    color: Color32,
    grosor: f32,
) {
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
    painter.add(egui::Shape::line(puntos, Stroke::new(grosor, color)));
}

/// Mini gráfica sin ejes para el widget compacto.
fn mini_grafica(ui: &mut egui::Ui, datos: &VecDeque<f32>, color: Color32, alto: f32) {
    let (resp, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), alto), Sense::hover());
    painter.rect_filled(resp.rect, 4.0, ui.visuals().extreme_bg_color);
    dibujar_serie(&painter, resp.rect.shrink(1.0), datos, color, 1.5);
}

/// Icono de "imagen en imagen" (PiP): un marco con una ventana pequeña abajo a la derecha.
/// La flecha apunta hacia la ventana pequeña para entrar y hacia fuera para salir.
fn icono_pip(painter: &egui::Painter, rect: Rect, color: Color32, entrar: bool) {
    let trazo = Stroke::new(1.4, color);
    let marco = rect.shrink2(Vec2::new(1.0, 2.0));
    painter.rect_stroke(marco, 2.0, trazo, StrokeKind::Inside);

    let pequena = Rect::from_min_max(
        Pos2::new(marco.center().x + 1.0, marco.center().y + 0.5),
        marco.max - Vec2::splat(2.5),
    );
    painter.rect_filled(pequena, 1.0, color);

    let esquina = marco.min + Vec2::splat(3.0);
    let destino = pequena.min - Vec2::splat(1.5);
    if entrar {
        painter.arrow(esquina, destino - esquina, trazo);
    } else {
        painter.arrow(destino, esquina - destino, trazo);
    }
}

/// Botón con el icono PiP; devuelve la respuesta para saber si se pulsó.
fn boton_pip(ui: &mut egui::Ui, rect: Rect, entrar: bool, ayuda: &str) -> egui::Response {
    let resp = ui
        .interact(rect, ui.id().with(("pip", entrar)), Sense::click())
        .on_hover_text(ayuda)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if resp.hovered() {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    icono_pip(ui.painter(), rect, color, entrar);
    resp
}

/// Una fila del widget compacto: etiqueta, porcentaje y mini gráfica.
fn fila_compacta(
    ui: &mut egui::Ui,
    etiqueta: &str,
    valor: f32,
    hist: &VecDeque<f32>,
    color: Color32,
) {
    const ALTO: f32 = 22.0;
    ui.horizontal(|ui| {
        ui.add_sized(
            [30.0, ALTO],
            egui::Label::new(RichText::new(etiqueta).strong()),
        );
        ui.add_sized(
            [48.0, ALTO],
            egui::Label::new(
                RichText::new(format!("{valor:.0} %"))
                    .color(color_carga(valor))
                    .strong(),
            ),
        );
        mini_grafica(ui, hist, color, ALTO);
    });
}

/// Recuadro de sección que ocupa todo el ancho disponible.
fn seccion(ui: &mut egui::Ui, contenido: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        contenido(ui);
    });
}

const TEXTO_OSCURO: Color32 = Color32::from_gray(20);
const TEXTO_CLARO: Color32 = Color32::WHITE;

/// Luminancia relativa de un color (fórmula WCAG).
fn luminancia(c: Color32) -> f32 {
    let canal = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * canal(c.r()) + 0.7152 * canal(c.g()) + 0.0722 * canal(c.b())
}

/// Relación de contraste WCAG entre dos colores (de 1 a 21).
fn contraste(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminancia(a), luminancia(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Texto oscuro o claro, el que más contraste tenga sobre `fondo`.
fn color_texto_sobre(fondo: Color32) -> Color32 {
    if contraste(fondo, TEXTO_OSCURO) >= contraste(fondo, TEXTO_CLARO) {
        TEXTO_OSCURO
    } else {
        TEXTO_CLARO
    }
}

/// Barra de carga de ancho `ancho` con el texto encima. El texto se dibuja dos veces,
/// recortado: con color de contraste sobre el relleno y con el color normal sobre el fondo,
/// para que siempre se lea aunque pase de una zona a otra.
fn barra_carga(ui: &mut egui::Ui, ancho: f32, p: f32, texto: &str) {
    const RADIO: f32 = 4.0;
    let alto = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ancho, alto), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter();
    let relleno_color = color_carga(p);
    painter.rect_filled(rect, RADIO, ui.visuals().extreme_bg_color);
    let x_corte = rect.left() + rect.width() * (p / 100.0).clamp(0.0, 1.0);
    let relleno = Rect::from_min_max(rect.min, Pos2::new(x_corte, rect.bottom()));
    let resto = Rect::from_min_max(Pos2::new(x_corte, rect.top()), rect.max);
    if relleno.width() > 0.0 {
        painter.rect_filled(relleno, RADIO, relleno_color);
    }

    let pos = Pos2::new(rect.left() + ui.spacing().item_spacing.x, rect.center().y);
    let fuente = egui::TextStyle::Button.resolve(ui.style());
    for (zona, color) in [
        (relleno, color_texto_sobre(relleno_color)),
        (resto, ui.visuals().text_color()),
    ] {
        painter
            .with_clip_rect(zona)
            .text(pos, Align2::LEFT_CENTER, texto, fuente.clone(), color);
    }
}

/// Barra de carga que ocupa todo el ancho disponible.
fn barra(ui: &mut egui::Ui, p: f32, texto: String) {
    barra_carga(ui, ui.available_width(), p, &texto);
}

impl Monitor {
    /// Vista completa. Devuelve `true` si se pulsó el botón de modo compacto.
    fn ui_completa(&self, ui: &mut egui::Ui) -> bool {
        let mut cambiar = false;
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
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(20.0, 16.0), Sense::hover());
                            let ayuda = format!(
                                "Modo compacto (siempre visible) · {}",
                                ui.ctx().format_shortcut(&ATAJO_COMPACTO)
                            );
                            if boton_pip(ui, rect, true, &ayuda).clicked() {
                                cambiar = true;
                            }
                            ui.add_space(4.0);
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
                        grafica(ui, &self.hist_cpu, COLOR_CPU);
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
                                    barra_carga(
                                        ui,
                                        ancho_barra,
                                        p,
                                        &format!("Núcleo {i}: {p:.0} %"),
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
                        grafica(ui, &self.hist_ram, COLOR_RAM);
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
        cambiar
    }

    /// Widget compacto: CPU y RAM con mini gráficas, sin barra de título y siempre encima.
    /// Se arrastra desde cualquier punto. Devuelve `true` si se pidió volver a la ventana completa.
    fn ui_compacta(&self, ui: &mut egui::Ui) -> bool {
        let mut salir = false;
        let visuals = ui.visuals().clone();
        egui::Frame::new()
            .fill(visuals.window_fill)
            .stroke(visuals.window_stroke)
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                // Sin texto seleccionable, para que arrastrar sobre las etiquetas mueva la ventana
                ui.style_mut().interaction.selectable_labels = false;

                let zona = ui.max_rect();
                let fondo = ui.interact(zona, ui.id().with("arrastre"), Sense::click_and_drag());
                if fondo.drag_started() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if fondo.double_clicked() {
                    salir = true;
                }

                let Some(m) = &self.actual else {
                    ui.centered_and_justified(|ui| ui.spinner());
                    return;
                };
                fila_compacta(ui, "CPU", m.cpu_global, &self.hist_cpu, COLOR_CPU);
                let p_ram = porcentaje(m.mem_usada, m.mem_total);
                fila_compacta(ui, "RAM", p_ram, &self.hist_ram, COLOR_RAM);

                // El botón para volver solo aparece al pasar el ratón, como en el PiP de macOS
                if ui.rect_contains_pointer(ui.clip_rect()) {
                    let rect = Rect::from_min_size(
                        Pos2::new(zona.right() - 20.0, zona.top() + 3.0),
                        Vec2::new(20.0, 16.0),
                    );
                    ui.painter()
                        .rect_filled(rect.expand(3.0), 4.0, visuals.window_fill);
                    let ayuda = format!(
                        "Volver a la ventana completa · {} o doble clic",
                        ui.ctx().format_shortcut(&ATAJO_COMPACTO)
                    );
                    if boton_pip(ui, rect, false, &ayuda).clicked() {
                        salir = true;
                    }
                }
            });
        salir
    }
}

impl eframe::App for Monitor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.recibir();

        let atajo = ui.ctx().input_mut(|i| i.consume_shortcut(&ATAJO_COMPACTO));
        let cambiar = if self.compacto {
            self.ui_compacta(ui)
        } else {
            self.ui_completa(ui)
        };
        if atajo || cambiar {
            self.alternar_modo(ui.ctx());
        }
    }

    /// Fondo transparente: así el widget compacto puede tener las esquinas redondeadas.
    /// En modo completo el panel central cubre toda la ventana, así que no se nota.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}

fn main() -> eframe::Result {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Monitor del sistema")
            .with_inner_size([460.0, 760.0])
            .with_min_inner_size(TAM_VENTANA_MIN)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "Monitor del sistema",
        opciones,
        Box::new(|cc| Ok(Box::new(Monitor::new(cc.egui_ctx.clone())))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_en_esquina_superior_derecha() {
        let pos = pos_inicial_widget(Some(Vec2::new(1440.0, 900.0)), Vec2::new(240.0, 70.0));
        assert_eq!(
            pos,
            Pos2::new(1440.0 - 240.0 - MARGEN_WIDGET, MARGEN_SUPERIOR_WIDGET)
        );
    }

    #[test]
    fn widget_sin_datos_del_monitor_va_arriba_a_la_izquierda() {
        let pos = pos_inicial_widget(None, Vec2::new(240.0, 70.0));
        assert_eq!(pos, Pos2::new(MARGEN_WIDGET, MARGEN_SUPERIOR_WIDGET));
    }

    #[test]
    fn widget_nunca_sale_por_la_izquierda_en_pantallas_diminutas() {
        let pos = pos_inicial_widget(Some(Vec2::new(100.0, 100.0)), Vec2::new(240.0, 70.0));
        assert_eq!(pos.x, 0.0);
    }

    #[test]
    fn texto_oscuro_sobre_amarillo_y_verde() {
        assert_eq!(color_texto_sobre(color_carga(60.0)), TEXTO_OSCURO);
        assert_eq!(color_texto_sobre(color_carga(10.0)), TEXTO_OSCURO);
    }

    #[test]
    fn texto_claro_sobre_fondo_oscuro() {
        assert_eq!(color_texto_sobre(Color32::from_gray(10)), TEXTO_CLARO);
    }

    #[test]
    fn contraste_minimo_legible_en_los_tres_colores_de_carga() {
        for p in [10.0, 60.0, 90.0] {
            let fondo = color_carga(p);
            let texto = color_texto_sobre(fondo);
            assert!(contraste(fondo, texto) >= 4.5, "carga {p} %");
        }
    }

    #[test]
    fn porcentaje_con_total_cero() {
        assert_eq!(porcentaje(5, 0), 0.0);
        assert_eq!(porcentaje(1, 4), 25.0);
    }

    #[test]
    fn formato_en_unidades_binarias() {
        assert_eq!(formato_bytes(512 * 1024 * 1024), "512 MiB");
        assert_eq!(formato_bytes(3 * 1024 * 1024 * 1024 / 2), "1.50 GiB");
    }
}
