# Arquitectura

> **English summary:** this document describes how the app is organized today (v0.3.1), the target architecture (a Rust library with a tiny `main.rs`, one folder per feature following the Model‑View‑Update pattern, platform access behind traits, and background collectors that publish only their latest value), why it was chosen, and the gradual migration plan. Read [Adding a feature](#8-cómo-añadir-una-funcionalidad) before contributing code.

Este documento explica **cómo está organizado el código, hacia dónde va y por qué**. Está pensado para cualquier persona que quiera entender el proyecto o contribuir, sin necesidad de conocer de antemano los patrones que se usan (hay un [glosario](#13-glosario) al final).

**Estado del documento:** en revisión (fase 0). Una vez aprobado, será la guía de la migración que empieza en la v0.4.0. La sección [Plan de migración](#11-plan-de-migración-gradual) indica en qué fase está el código en cada momento.

---

## Índice

1. [Principios](#1-principios)
2. [Estado actual (v0.3.1)](#2-estado-actual-v031)
3. [Por qué cambiar la arquitectura](#3-por-qué-cambiar-la-arquitectura)
4. [Opciones consideradas](#4-opciones-consideradas)
5. [Arquitectura objetivo](#5-arquitectura-objetivo)
6. [Estructura de carpetas](#6-estructura-de-carpetas)
7. [Anatomía de una funcionalidad](#7-anatomía-de-una-funcionalidad)
8. [Cómo añadir una funcionalidad](#8-cómo-añadir-una-funcionalidad)
9. [Reglas de dependencia](#9-reglas-de-dependencia)
10. [Concurrencia y flujo de datos](#10-concurrencia-y-flujo-de-datos)
11. [Plan de migración gradual](#11-plan-de-migración-gradual)
12. [Estrategia de tests](#12-estrategia-de-tests)
13. [Glosario](#13-glosario)
14. [Referencias](#14-referencias)

---

## 1. Principios

Estas reglas guían todas las decisiones de arquitectura:

1. **`main.rs` no crece.** Solo arranca la ventana. Todo el código vive en la librería (`lib.rs` y sus módulos).
2. **La lógica no depende de egui.** Qué hacer cuando el usuario pulsa algo o llega un dato nuevo se decide en funciones puras que se pueden probar sin abrir ninguna ventana.
3. **Las vistas solo dibujan.** Leen el estado y devuelven las acciones del usuario; no cambian el estado ni llaman al sistema operativo.
4. **La lectura del sistema nunca ocurre en el hilo de la interfaz.** La interfaz jamás espera a `sysinfo` ni a `listeners`.
5. **Gana el último valor.** Un monitor en tiempo real solo necesita la lectura más reciente; nunca se acumulan lecturas viejas.
6. **El acceso al sistema operativo está detrás de *traits*.** Así los tests usan datos falsos y controlados, y el código real queda aislado en un solo sitio.
7. **Una funcionalidad, una carpeta.** Añadir algo nuevo es sobre todo crear archivos nuevos, no editar los existentes.
8. **Sin complejidad que no se necesite.** Sin async, sin persistencia, sin shell ni comandos externos y sin dependencias nuevas salvo decisión explícita y justificada.

---

## 2. Estado actual (v0.3.1)

```
src/
├── main.rs      (1189 líneas)  hilo lector, estado, vistas, diálogo, cierres, dibujo y main()
└── puertos.rs   ( 624 líneas)  dominio de puertos: filtro, lectura, terminar procesos y tests
```

**Cómo funciona hoy:**

```
Hilo lector (cada 500 ms)                       Hilo de interfaz (eframe)
  sysinfo: CPU, RAM, todos los procesos           Monitor (24 campos de estado)
  cada 2 s: puertos::leer ─────┐                    ├─ recibir()  ← vacía el canal
                               ▼                    ├─ vista_sistema / vista_puertos / ui_compacta
                      mpsc::channel() (sin límite) ─┘ └─ diálogo, cierres (hilo por cierre)
```

- `src/puertos.rs` ya sigue los principios: su lógica es pura y está bien probada (19 tests, incluidos procesos reales).
- `src/main.rs` mezcla todas las demás responsabilidades.

---

## 3. Por qué cambiar la arquitectura

No es una preferencia estética: el propio historial del proyecto muestra los problemas.

**`main.rs` crece con cada versión:**

| Versión | `main.rs` | Qué se añadió |
|---|---|---|
| 0.1.0 | 230 líneas | Monitor básico |
| 0.2.0 | 721 | Modo compacto, hilo lector, ejes de las gráficas |
| 0.2.1 | 799 | Barras con contraste |
| 0.3.0 | 1183 | Pestaña de puertos, diálogo y cierres |
| 0.3.1 | 1189 | Primeras contribuciones externas |

Es decir, **×5,2 en cuatro versiones**. La hoja de ruta añade red, disco, temperaturas, contexto de proyecto, limpieza de carpetas de desarrollo y Docker; con la estructura actual `main.rs` llegaría a varios miles de líneas.

**Síntomas que ya aparecen:**

- **Conflictos entre contribuciones.** Las dos primeras contribuciones externas (#33 y #34) modificaron líneas vecinas del mismo archivo. Con más contribuidores, los conflictos serían constantes.
- **Lógica sin tests.** La máquina de estados de los cierres ("Cerrando…", "No responde", filas que se quitan, avisos) está mezclada con el dibujo y no se puede probar sin abrir la ventana (issue #28). Varios de los detalles menores encontrados en la revisión de la pestaña de puertos (#21, #22, #23 y el ya corregido #24) vienen de ahí.
- **Acumulación de lecturas.** Cuando la ventana está minimizada o tapada por completo, eframe no llama a `App::ui`, que es donde se vacía el canal. El canal `mpsc` no tiene límite, así que acumula una lectura cada 500 ms mientras la ventana está oculta.

**Por qué ahora:** la deuda de arquitectura es la más cara de pagar y crece con el tiempo, porque cada funcionalidad nueva añade más código enredado. Con unas 1800 líneas, CI, releases y contribuidores externos, el proyecto ya tiene el tamaño en el que conviene ordenarlo, y todavía es pequeño para hacerlo con poco riesgo. Rust ayuda: el compilador detecta casi todo lo que se rompe al mover código.

---

## 4. Opciones consideradas

| Opción | Ventajas | Inconvenientes | Decisión |
|---|---|---|---|
| **Seguir igual** | Ningún trabajo ahora | Los problemas de la sección 3 crecen con cada funcionalidad | Descartada |
| **Solo dividir `main.rs` en archivos** | Poco trabajo | La lógica sigue mezclada con la interfaz y sin tests; el estado sigue siendo un único struct que todos tocan | Insuficiente, pero es el primer paso |
| **MVU único** (un solo `enum Accion` y una sola función `actualizar`) | Lógica pura y probable | `actualizar` se convierte en el nuevo archivo gigante: un `match` que crece con cada funcionalidad | Descartada |
| **MVU por funcionalidad + librería + *traits* de plataforma** | `main.rs` fijo; cada funcionalidad crece en su carpeta; lógica y lectura probables con datos falsos | Algo más de estructura (enums de reparto y *traits*) | **Elegida** |
| **Workspace con varios crates** (como Rerun) | Límites muy estrictos y compilación incremental | Excesivo para el tamaño actual | Aplazada; los módulos por funcionalidad permiten pasar a crates más adelante si hace falta |

**Por qué MVU:** es el patrón que recomiendan egui (mantener la lógica fuera del código que dibuja) y Ratatui, y el que impone iced. **Por qué por funcionalidad:** un MVU único acaba siendo un `match` gigante; la propia comunidad de Elm advierte del problema contrario, anidar demasiado, así que aquí solo hay **dos niveles**: la app, que reparte, y cada funcionalidad, que decide.

---

## 5. Arquitectura objetivo

La aplicación se organiza en cuatro capas:

| Capa | Qué contiene | ¿Usa egui? | ¿Usa el sistema operativo? |
|---|---|---|---|
| **Arranque** | `main.rs` | Solo para abrir la ventana | No |
| **App** | Estado global, acciones de primer nivel, reparto y ejecución de efectos | Sí (ciclo de eframe) | No, lo hace a través de efectos |
| **Funcionalidades** | Cada una: estado, acciones, lógica pura, vista y recolector | Solo en `vista.rs` | Solo a través de *traits* |
| **Plataforma** | Implementaciones reales de los *traits* (`sysinfo`, `listeners`, señales) | No | **Sí, es el único sitio** |

**El ciclo de cada fotograma:**

```
            ┌───────────── hilo lector ─────────────┐
            │ recolectores (cada uno con su cadencia) │
            │   └─ publican su última lectura ─────┐  │
            └──────────────────────────────────────┼──┘
                                                   ▼
                                            Buzón (último valor)
                                                   │ tomar()
┌──────────────────────── hilo de interfaz ────────┼───────────────────────────┐
│                                                  ▼                           │
│  Accion::…::NuevaLectura ─▶ actualizar(estado, acción) ─▶ Vec<Efecto>        │
│                                  ▲                            │              │
│                                  │ acciones del usuario       ▼              │
│  vista(&estado) ─ dibuja con egui ┘                 ejecutor de efectos       │
│                                                      ├─ comandos de ventana  │
│                                                      └─ lanzar cierre (hilo) │
└──────────────────────────────────────────────────────────────────────────────┘
                                                             │
                                          resultado del cierre (canal de eventos)
                                                             ▼
                                          Accion::Puertos::CierreTerminado(…)
```

**Piezas clave:**

- **Acción:** algo que ocurrió: el usuario pulsó un botón o llegó una lectura.
- **`actualizar`:** función **pura** que, dada una acción, cambia el estado y devuelve una lista de **efectos**. No dibuja ni llama al sistema.
- **Efecto:** algo que hay que hacer fuera: enviar un comando a la ventana o lanzar el cierre de un proceso. Lo ejecuta la capa App, nunca la lógica.
- **Recolector:** código del hilo lector que obtiene los datos de una funcionalidad con su propia cadencia y publica solo la última lectura.

---

## 6. Estructura de carpetas

Estructura objetivo al terminar la migración:

```
src/
├── main.rs                      Arranca la ventana. ~10 líneas. No crece.
├── lib.rs                       Declara los módulos de la librería.
│
├── app/                         Nivel superior: reparte, no decide.
│   ├── mod.rs                   impl eframe::App: tomar lecturas → actualizar → dibujar → efectos
│   ├── estado.rs                struct Estado { sistema, puertos, compacto, pestana, … }
│   ├── accion.rs                enum Accion { Sistema(…), Puertos(…), Compacto(…), CambiarPestana(…) }
│   ├── actualizar.rs            Reparte cada acción a su funcionalidad
│   └── efectos.rs               enum Efecto y su ejecutor (ventana, hilos de cierre)
│
├── funcionalidades/             Una carpeta por funcionalidad, todas con la misma forma.
│   ├── sistema/                 CPU, RAM, núcleos, top de procesos, histórico
│   │   ├── mod.rs
│   │   ├── estado.rs
│   │   ├── accion.rs
│   │   ├── actualizar.rs        (con tests)
│   │   ├── recolector.rs        (con tests con plataforma falsa)
│   │   └── vista.rs
│   ├── puertos/                 Pestaña Puertos
│   │   ├── mod.rs
│   │   ├── dominio.rs           el actual puertos.rs: filtro, preparar, coincide, mensajes (con tests)
│   │   ├── cierre.rs            terminar() con comprobación de identidad (con tests)
│   │   ├── estado.rs · accion.rs · actualizar.rs · recolector.rs · vista.rs
│   └── compacto/                Widget siempre visible
│       ├── mod.rs · estado.rs · accion.rs · actualizar.rs · vista.rs
│
├── lector/                      Hilo en segundo plano.
│   ├── mod.rs                   Bucle que recorre los recolectores; señal de parada
│   ├── recolector.rs            trait Recolector
│   └── buzon.rs                 Buzon<T>: solo guarda la última lectura (con tests)
│
├── plataforma/                  ÚNICO lugar que habla con el sistema operativo.
│   ├── mod.rs                   traits: FuenteSistema, FuentePuertos, ControlProcesos
│   ├── real.rs                  Implementaciones con sysinfo y listeners
│   └── falsa.rs                 Implementaciones para tests (solo con cfg(test))
│
└── ui/                          Piezas visuales compartidas (sin lógica de negocio).
    ├── mod.rs
    ├── marco.rs                 Cabecera y pestañas de la ventana completa
    ├── widgets.rs               Gráficas, barras con contraste, icono PiP, secciones
    └── tema.rs                  Colores y constantes visuales
```

**Futuras funcionalidades** (red, disco, contexto de proyecto, limpieza, Docker) se añaden como carpetas nuevas dentro de `funcionalidades/`. La v0.4.0, por ejemplo, añadirá `funcionalidades/contexto/`, que resuelve PID → cwd → raíz del servicio → raíz del repositorio → manifiestos → runtime/framework → rama y commit de Git, con caché.

---

## 7. Anatomía de una funcionalidad

Todas las funcionalidades tienen la misma forma. Ejemplo simplificado con Puertos:

```rust
// estado.rs — lo que la funcionalidad sabe. Sin egui.
pub struct Estado {
    pub lista: Option<ResultadoLectura>,
    pub busqueda: String,
    pub en_curso: HashMap<(u16, u32), EstadoCierre>,
    pub confirmacion: Option<Confirmacion>,
    pub aviso: Option<Aviso>,
}

// accion.rs — todo lo que puede ocurrir.
pub enum Accion {
    NuevaLectura(ResultadoLectura),
    CambiarBusqueda(String),
    PedirCierre(PuertoInfo, bool),
    ConfirmarCierre,
    CancelarCierre,
    CierreTerminado(PuertoInfo, bool, ResultadoCierre),
}

// actualizar.rs — la lógica. Pura: sin egui y sin sistema operativo.
pub fn actualizar(estado: &mut Estado, accion: Accion) -> Vec<Efecto> { /* … */ }

// vista.rs — solo dibuja y devuelve lo que hizo el usuario.
pub fn vista(ui: &mut egui::Ui, estado: &Estado) -> Vec<Accion> { /* … */ }

// recolector.rs — se ejecuta en el hilo lector, con su propia cadencia.
pub struct RecolectorPuertos<F: FuentePuertos> { /* … */ }
impl<F: FuentePuertos> Recolector for RecolectorPuertos<F> { /* lee cada 2 s y publica en su buzón */ }
```

Y un test de la lógica, sin ventana:

```rust
#[test]
fn si_no_responde_se_ofrece_forzar() {
    let mut e = Estado::default();
    actualizar(&mut e, Accion::PedirCierre(p.clone(), false));
    let efectos = actualizar(&mut e, Accion::ConfirmarCierre);
    assert_eq!(efectos, vec![Efecto::LanzarCierre(p.clone(), false)]);
    actualizar(&mut e, Accion::CierreTerminado(p, false, ResultadoCierre::NoResponde));
    assert_eq!(e.en_curso[&(4200, 10)], EstadoCierre::NoResponde);
}
```

---

## 8. Cómo añadir una funcionalidad

1. **Crea la carpeta** `src/funcionalidades/<nombre>/` copiando la forma de una existente.
2. **Escribe primero los tests** de `actualizar.rs` y, si lee datos, del recolector con la plataforma falsa.
3. **Si necesitas datos nuevos del sistema**, añade un método a un *trait* de `plataforma/`, su implementación real y la falsa. Ningún otro módulo debe llamar a `sysinfo` ni a `listeners`.
4. **Conéctala en la capa App:**
   - una variante en `app/accion.rs`;
   - una rama en `app/actualizar.rs`;
   - un campo en `app/estado.rs`;
   - su pestaña en `ui/marco.rs`, si la tiene;
   - su recolector en la lista del lector, si lee datos.

   Son pocas líneas de reparto, sin lógica.
5. **Nada más en `main.rs`.**

---

## 9. Reglas de dependencia

Quién puede usar a quién:

```
main.rs ──▶ app ──▶ funcionalidades ──▶ plataforma (solo los traits)
             │            │
             │            └──▶ ui (widgets, tema)
             └──▶ lector ──▶ plataforma (implementaciones reales)
```

- **`funcionalidades/*` no dependen unas de otras.** Si dos necesitan compartir datos (por ejemplo, contexto de proyecto y puertos), se comparten tipos de un módulo común o se combinan en la capa App.
- **`actualizar.rs` y `estado.rs` nunca importan egui.**
- **`vista.rs` nunca llama a `plataforma/`** ni lanza hilos: devuelve acciones.
- **Solo `plataforma/real.rs` importa `sysinfo` y `listeners`.**
- **`ui/` no conoce ninguna funcionalidad:** solo ofrece piezas de dibujo.

---

## 10. Concurrencia y flujo de datos

- **Hilo de interfaz:** eframe. Dibuja y ejecuta `actualizar`, que es instantáneo.
- **Hilo lector:** uno solo, en segundo plano. Recorre los recolectores; cada uno decide si le toca leer según su cadencia (sistema: 500 ms; puertos: 2 s y solo con la ventana completa). Tiene una señal de parada para los tests.
- **Buzones de último valor:** cada recolector publica en su propio `Buzon<T>` (`Mutex<Option<T>>`). Publicar **sustituye** la lectura anterior y la interfaz la **toma**, dejando el buzón vacío. Consecuencias:
  - nunca se acumulan lecturas, aunque la ventana esté oculta horas;
  - el histórico de las gráficas lo mantiene el recolector del sistema, así que no se pierden puntos aunque la interfaz no consuma;
  - una lectura de puertos solo se aplica una vez; si la interfaz quitó una fila tras un cierre, no reaparece hasta la siguiente lectura real.
- **Canal de eventos para resultados de cierre:** `mpsc`. Son eventos que no se pueden perder y solo se producen al pulsar un botón.
- **Hilos de cierre:** uno por cierre, porque `terminar()` espera hasta 3 s. Los lanza el ejecutor de efectos.
- **Sin mutex compartidos de larga duración**, sin async y sin Tokio.

---

## 11. Plan de migración gradual

La migración se hace **por fases pequeñas**. Cada fase es un PR hacia `develop` que no cambia el comportamiento visible salvo cuando se indica, pasa el CI en los tres sistemas y se prueba a mano antes de fusionarse.

| Fase | Qué se hace | ¿Cambia el comportamiento? | Tests nuevos | Estado |
|---|---|---|---|---|
| **0** | Este documento | No | — | En revisión |
| **1** | `lib.rs` y `main.rs` mínimo; mover código a `app/`, `ui/`, `lector/` y `funcionalidades/` **sin cambiar lógica** (commits solo de movimiento) | No | No: se mueven los 27 existentes | Pendiente |
| **2** | Buzones de último valor y recolectores; histórico en el recolector; *traits* de plataforma con implementación falsa | Solo interno: la memoria deja de crecer con la ventana oculta | Buzón, histórico, top de procesos, cadencia e integración del lector | Pendiente |
| **3** | Piloto MVU en Puertos: estado, acciones, `actualizar` y vista que devuelve acciones | No | Máquina de estados de los cierres (cierra #28) | Pendiente |
| **4** | Sistema y compacto al mismo patrón; `app/` solo reparte | No | Lógica de pestañas y modo compacto | Pendiente |
| **5** | v0.4.0: `funcionalidades/contexto/` nace con el patrón | Sí: funcionalidad nueva | Detección de raíz, manifiestos y Git con sistema de archivos falso | Pendiente |

**Cómo se evita romper nada en cada fase:**

- **Separar "mover" de "cambiar".** Los commits de movimiento no modifican el cuerpo de ninguna función; se revisan con `git diff --color-moved`.
- **Mismos textos:** se comparan todas las cadenas literales de `src/` antes y después.
- **Mismas constantes:** cadencias (500 ms, 2 s), espera de cierre (3 s), histórico (120 puntos), top 8, atajos y tamaños.
- **Prueba manual en macOS antes de cada PR**, además del CI en macOS, Windows y Linux.
- **Actualizar las rutas** citadas en los issues abiertos al terminar cada fase que mueva archivos.

Esta tabla se actualiza al completar cada fase.

---

## 12. Estrategia de tests

| Nivel | Qué prueba | Dónde | Ejemplos |
|---|---|---|---|
| **Lógica pura** | `actualizar`, dominio, buzón, histórico | Junto al código (`#[cfg(test)]`) | Filtro de puertos, máquina de cierres, pestañas |
| **Recolectores con plataforma falsa** | Cadencias, errores, casos raros | Junto al recolector | Error de permisos, 500 puertos, proceso que no responde |
| **Plataforma real** | Que `sysinfo`, `listeners` y las señales funcionan de verdad en cada sistema | `plataforma/` y el CI en tres sistemas | Leer un puerto real, terminar un proceso hijo real |
| **Interfaz** | Que se dibuja y responde como se espera | Prueba manual (lista en cada PR) | Pestañas, diálogo, widget |

**Hueco conocido:** el dibujo en sí no tiene tests automáticos. Con esta arquitectura es pequeño, porque las vistas no deciden nada. Para cubrirlo existe `egui_kittest`, la librería oficial de egui para tests de interfaz; añadirla sería una dependencia nueva (solo de desarrollo) y se decidirá aparte.

Comandos: `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.

---

## 13. Glosario

- **MVU (Model‑View‑Update), o arquitectura Elm:** forma de organizar una interfaz en tres partes: el **estado** (qué se sabe), la **actualización** (cómo cambia el estado ante cada acción) y la **vista** (cómo se dibuja el estado).
- **Acción:** algo que ocurrió, como un clic o una lectura nueva. Es un valor de un `enum`.
- **Efecto:** algo que la lógica pide hacer fuera de ella, como lanzar un cierre o mover la ventana. La lógica lo devuelve y otra parte lo ejecuta.
- **Función pura:** su resultado depende solo de sus entradas y no toca nada externo (pantalla, sistema, archivos). Por eso se prueba fácilmente.
- **Trait:** en Rust, un contrato del tipo "cualquier cosa que sepa leer puertos". Permite usar la versión real en la app y una falsa en los tests.
- **Recolector:** pieza del hilo lector que obtiene los datos de una funcionalidad con su propia cadencia.
- **Buzón de último valor:** contenedor que solo guarda la lectura más reciente; publicar sustituye la anterior.
- **Cadencia:** cada cuánto se lee un dato (500 ms, 2 s…).

---

## 14. Referencias

- [Gestión de estado en egui (DeepWiki)](https://deepwiki.com/emilk/egui/2.5-memory-and-state-management): mantener la lógica fuera del código que dibuja y usar la arquitectura Elm con estados complejos.
- [Arquitectura de Rerun](https://github.com/rerun-io/rerun/blob/main/ARCHITECTURE.md): un proyecto grande con egui organizado en piezas separadas y documentado en un `ARCHITECTURE.md`.
- [Arquitectura de iced](https://book.iced.rs/architecture.html) y [La arquitectura Elm en Ratatui](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/): el patrón MVU en Rust.
- [Xilem: una arquitectura de UI para Rust (Raph Levien)](https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html) y [elm-taco-donut](https://github.com/madasebrof/elm-taco-donut): límites del MVU único y del anidamiento excesivo.
- [Big Ball of Mud (DevIQ)](https://deviq.com/antipatterns/big-ball-of-mud/): por qué la deuda de arquitectura se encarece con el tiempo.
- [Fearless Refactoring (RustLab 2024)](https://rustlab.it/talks/fearless-refactoring-and-the-art-of-argument-free-rust): por qué refactorizar en Rust es más seguro.
