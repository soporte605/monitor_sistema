# Pestaña «Puertos»: plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendada) o superpowers:executing-plans para ejecutar este plan tarea a tarea. Los pasos usan casillas (`- [ ]`) para el seguimiento.

**Objetivo:** añadir a la ventana completa una pestaña «Puertos» que lista los puertos TCP de desarrollo en escucha, muestra qué proceso ocupa cada uno y permite terminarlo con confirmación.

**Arquitectura:** un módulo nuevo `src/puertos.rs` contiene el modelo, las funciones puras (filtro, unión IPv4/IPv6, búsqueda, mensajes), la lectura real (`listeners` + `sysinfo`) y la función para terminar procesos. El hilo lector existente lee los puertos cada 2 s mientras la ventana completa está visible y los envía en la `Muestra`. La interfaz (`main.rs`) añade pestañas, la vista de puertos y un diálogo modal; cada cierre se ejecuta en un hilo aparte que devuelve el resultado por un canal.

**Tecnologías:** Rust 2024, eframe/egui 0.36, egui_extras 0.36, sysinfo 0.39.6, listeners 0.6.1 (nueva).

**Diseño:** [`docs/diseño/2026-09-25-puertos.md`](2026-09-25-puertos.md). Leerlo antes de empezar; este plan lo implementa.

## Restricciones globales

- Dependencia nueva autorizada: solo `listeners = "0.6.1"`. No añadir ninguna otra.
- Solo sockets **TCP en estado Listen**.
- Filtro: proceso del mismo usuario que la app **y** puerto ≥ 1024 **y** ejecutable fuera de `/System/`, `/usr/libexec/`, `/usr/sbin/`, `/usr/lib/systemd/`, `C:\Windows\` (sin distinguir mayúsculas). La app nunca se lista a sí misma.
- Lectura de puertos cada **2 s**. Espera tras SIGTERM: **3 s**.
- Textos de la interfaz y comentarios en español. Mensajes exactos:
  - «No hay puertos de desarrollo en uso.»
  - «¿No ves tu puerto? Puede estar ocupado por un servicio del sistema o de otro usuario.»
  - «Ningún puerto coincide con «…».»
  - «Si tiene trabajo sin guardar, se perderá.»
  - «El proceso no respondió. ¿Forzar el cierre? No podrá guardar nada.»
  - «✓ Puerto N liberado»
  - «El proceso ya había terminado.»
  - «No tienes permiso para terminar este proceso.»
  - «El proceso ya no existe o cambió. La lista se ha actualizado.»
- Anchos adaptables: solo el puerto y el botón de acción tienen ancho fijo.
- Antes de cada commit: `cargo fmt`, `cargo clippy --all-targets` sin advertencias nuevas, `cargo test` en verde. Conventional Commits en español, **sin líneas de atribución**.
- Comandos de cargo: si `cargo` no está en el PATH, ejecutar antes `source "$HOME/.cargo/env"`.

## Corrección al diseño

El diseño dice que los puertos se leen «solo mientras la pestaña está visible», pero también pide mostrar «Puertos · N» para verlo **sin entrar en la pestaña**. Ambas cosas son incompatibles. Este plan lee los puertos cada 2 s **mientras la ventana completa está visible (en cualquier pestaña) y nunca en modo compacto**. La tarea 7 actualiza el documento de diseño.

## Focos de revisión

Casos que el diseño implica y que ningún test obvio cubriría:

1. **Un proceso cuya línea de comandos llega vacía** (macOS la oculta a veces): debe mostrarse la ruta del ejecutable o el nombre, nunca una línea en blanco. Test en la tarea 1 (`comando_legible`).
2. **Dos procesos distintos en el mismo puerto** (por ejemplo, un padre y su hijo compartiendo el socket): deben salir dos filas, no unirse en una. Test en la tarea 1.
3. **Búsqueda con espacios o mayúsculas** («  NG  »): debe encontrar `ng serve`. Test en la tarea 1.
4. **El proceso muere entre la lista y el clic en «Terminar»**: debe decir «El proceso ya había terminado.» y no dar error. Test en la tarea 3.
5. **PID reutilizado por otro proceso**: no debe tocarse; resultado «cambió». Test en la tarea 3.

---

## Estructura de archivos

| Archivo | Responsabilidad |
|---|---|
| `Cargo.toml` | Añade `listeners`. |
| `src/puertos.rs` (nuevo) | Modelo `PuertoInfo`/`Candidato`, filtro, unión, orden, búsqueda, lectura real, terminar procesos, mensajes de resultado y sus tests. |
| `src/main.rs` | `mod puertos;`, hilo lector con bandera y cadencia, estado de la interfaz (pestaña, búsqueda, cierres en curso, aviso, confirmación), pestañas, vista de puertos y diálogo modal. |
| `CHANGELOG.md`, `README.md`, `README.en.md` | Documentación de la funcionalidad. |
| `docs/diseño/2026-09-25-puertos.md` | Corrección de la cadencia de lectura. |

---

### Tarea 1: dependencia y funciones puras del módulo

**Archivos:**
- Modificar: `Cargo.toml` (sección `[dependencies]`)
- Crear: `src/puertos.rs`
- Modificar: `src/main.rs` (declaración del módulo, después de los `use`)

**Interfaces:**
- Produce (en `src/puertos.rs`):
  - `pub struct PuertoInfo { pub puerto: u16, pub pid: u32, pub nombre: String, pub comando: String, pub inicio: u64, pub es_dev: bool }` con `#[derive(Debug, Clone, PartialEq)]`
  - `pub struct Candidato { pub puerto: u16, pub pid: u32, pub nombre: String, pub ruta: String, pub comando: String, pub inicio: u64, pub es_del_usuario: bool }` con `#[derive(Debug, Clone)]`
  - `pub fn en_carpeta_del_sistema(ruta: &str) -> bool`
  - `pub fn es_de_desarrollo(c: &Candidato) -> bool`
  - `pub fn es_herramienta_dev(nombre: &str) -> bool`
  - `pub fn comando_legible(cmd: &[String], ruta: &str, nombre: &str) -> String`
  - `pub fn preparar(candidatos: Vec<Candidato>, pid_propio: u32) -> Vec<PuertoInfo>`
  - `pub fn coincide(p: &PuertoInfo, busqueda: &str) -> bool`

- [ ] **Paso 1: añadir la dependencia**

En `Cargo.toml`, dentro de `[dependencies]`, añadir la línea:

```toml
listeners = "0.6.1"
```

- [ ] **Paso 2: declarar el módulo**

En `src/main.rs`, justo después de `use sysinfo::{ProcessesToUpdate, System};`, añadir:

```rust
// Mientras la interfaz no use el módulo (tareas 1 a 5), se evita el aviso de código sin usar.
// La tarea 6 quita este atributo.
#[cfg_attr(not(test), allow(dead_code))]
mod puertos;
```

- [ ] **Paso 3: escribir los tests que fallan**

Crear `src/puertos.rs` con solo esto:

```rust
//! Pestaña «Puertos»: qué proceso ocupa cada puerto de desarrollo y cómo terminarlo.

#[cfg(test)]
mod tests {
    use super::*;

    fn candidato(puerto: u16, pid: u32, nombre: &str, ruta: &str) -> Candidato {
        Candidato {
            puerto,
            pid,
            nombre: nombre.to_string(),
            ruta: ruta.to_string(),
            comando: format!("{nombre} servidor"),
            inicio: 1000,
            es_del_usuario: true,
        }
    }

    #[test]
    fn proceso_propio_en_puerto_alto_entra() {
        assert!(es_de_desarrollo(&candidato(4200, 10, "node", "/usr/local/bin/node")));
    }

    #[test]
    fn proceso_de_otro_usuario_no_entra() {
        let mut c = candidato(4200, 10, "node", "/usr/local/bin/node");
        c.es_del_usuario = false;
        assert!(!es_de_desarrollo(&c));
    }

    #[test]
    fn puerto_menor_que_1024_no_entra() {
        assert!(!es_de_desarrollo(&candidato(80, 10, "node", "/usr/local/bin/node")));
        assert!(es_de_desarrollo(&candidato(1024, 10, "node", "/usr/local/bin/node")));
    }

    #[test]
    fn ejecutables_del_sistema_no_entran() {
        assert!(en_carpeta_del_sistema("/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter"));
        assert!(en_carpeta_del_sistema("/usr/libexec/rapportd"));
        assert!(en_carpeta_del_sistema("/usr/sbin/sshd"));
        assert!(en_carpeta_del_sistema("/usr/lib/systemd/systemd-resolved"));
        assert!(en_carpeta_del_sistema(r"C:\Windows\System32\svchost.exe"));
        assert!(en_carpeta_del_sistema(r"c:\windows\system32\svchost.exe"));
        assert!(!en_carpeta_del_sistema("/usr/local/bin/node"));
        assert!(!en_carpeta_del_sistema(r"C:\Program Files\nodejs\node.exe"));
    }

    #[test]
    fn airplay_propio_en_5000_no_entra() {
        let c = candidato(
            5000,
            10,
            "ControlCenter",
            "/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter",
        );
        assert!(!es_de_desarrollo(&c));
    }

    #[test]
    fn reconoce_herramientas_de_desarrollo() {
        for n in ["node", "node.exe", "NODE", "python3", "python3.12", "java", "deno", "bun", "dotnet", "ruby", "php", "go"] {
            assert!(es_herramienta_dev(n), "{n}");
        }
        for n in ["Safari", "Spotify", "ControlCenter"] {
            assert!(!es_herramienta_dev(n), "{n}");
        }
    }

    #[test]
    fn comando_vacio_usa_la_ruta_o_el_nombre() {
        let args = vec!["node".to_string(), "server.js".to_string()];
        assert_eq!(comando_legible(&args, "/usr/local/bin/node", "node"), "node server.js");
        assert_eq!(comando_legible(&[], "/usr/local/bin/node", "node"), "/usr/local/bin/node");
        assert_eq!(comando_legible(&[], "", "node"), "node");
    }

    #[test]
    fn une_ipv4_e_ipv6_del_mismo_proceso() {
        let lista = preparar(
            vec![
                candidato(4200, 10, "node", "/usr/local/bin/node"),
                candidato(4200, 10, "node", "/usr/local/bin/node"),
            ],
            1,
        );
        assert_eq!(lista.len(), 1);
    }

    #[test]
    fn dos_procesos_en_el_mismo_puerto_son_dos_filas() {
        let lista = preparar(
            vec![
                candidato(4200, 10, "node", "/usr/local/bin/node"),
                candidato(4200, 11, "node", "/usr/local/bin/node"),
            ],
            1,
        );
        assert_eq!(lista.len(), 2);
    }

    #[test]
    fn ordena_por_puerto_y_excluye_la_propia_app() {
        let lista = preparar(
            vec![
                candidato(8080, 12, "java", "/usr/bin/java"),
                candidato(3000, 11, "node", "/usr/local/bin/node"),
                candidato(4200, 1, "monitor_sistema", "/Applications/Monitor.app/m"),
            ],
            1,
        );
        let puertos: Vec<u16> = lista.iter().map(|p| p.puerto).collect();
        assert_eq!(puertos, vec![3000, 8080]);
        assert!(lista.iter().all(|p| p.es_dev));
    }

    #[test]
    fn busqueda_por_numero_nombre_y_comando() {
        let mut c = candidato(4200, 10, "node", "/usr/local/bin/node");
        c.comando = "node /proyecto/node_modules/.bin/ng serve --port 4200".to_string();
        let p = &preparar(vec![c], 1)[0];
        assert!(coincide(p, ""));
        assert!(coincide(p, "4200"));
        assert!(coincide(p, "NODE"));
        assert!(coincide(p, "  ng serve  "));
        assert!(!coincide(p, "python"));
    }
}
```

- [ ] **Paso 4: comprobar que fallan**

Ejecutar: `cargo test puertos`
Esperado: error de compilación `cannot find function` / `cannot find struct` para `Candidato`, `es_de_desarrollo`, etc.

- [ ] **Paso 5: implementar**

Añadir en `src/puertos.rs`, **encima** del bloque `#[cfg(test)]`:

```rust
use std::collections::HashSet;

/// Un puerto de desarrollo en escucha, listo para mostrar.
#[derive(Debug, Clone, PartialEq)]
pub struct PuertoInfo {
    pub puerto: u16,
    pub pid: u32,
    pub nombre: String,
    /// Línea de comandos (o la ruta o el nombre si el sistema no la da).
    pub comando: String,
    /// Hora de inicio del proceso, para no confundirlo con otro que reutilice el PID.
    pub inicio: u64,
    /// Es una herramienta de desarrollo conocida (solo informativo).
    pub es_dev: bool,
}

/// Un socket en escucha antes de filtrar, con los datos necesarios para decidir.
#[derive(Debug, Clone)]
pub struct Candidato {
    pub puerto: u16,
    pub pid: u32,
    pub nombre: String,
    pub ruta: String,
    pub comando: String,
    pub inicio: u64,
    pub es_del_usuario: bool,
}

/// Carpetas de programas del sistema (macOS, Linux y Windows), en minúsculas.
const CARPETAS_DEL_SISTEMA: &[&str] = &[
    "/system/",
    "/usr/libexec/",
    "/usr/sbin/",
    "/usr/lib/systemd/",
    r"c:\windows\",
];

/// Herramientas de desarrollo conocidas (nombre sin extensión, en minúsculas).
const HERRAMIENTAS_DEV: &[&str] = &["node", "deno", "bun", "java", "dotnet", "ruby", "php", "go"];

/// El ejecutable está en una carpeta del sistema.
pub fn en_carpeta_del_sistema(ruta: &str) -> bool {
    let ruta = ruta.to_lowercase();
    CARPETAS_DEL_SISTEMA.iter().any(|c| ruta.starts_with(c))
}

/// Un puerto cuenta como «de desarrollo» si el proceso es del usuario,
/// el puerto no es privilegiado y el ejecutable no es del sistema.
pub fn es_de_desarrollo(c: &Candidato) -> bool {
    c.es_del_usuario && c.puerto >= 1024 && !en_carpeta_del_sistema(&c.ruta)
}

/// Reconoce herramientas de desarrollo por su nombre (`node`, `node.exe`, `python3.12`…).
pub fn es_herramienta_dev(nombre: &str) -> bool {
    let nombre = nombre.to_lowercase();
    let base = nombre.strip_suffix(".exe").unwrap_or(&nombre);
    base.starts_with("python") || HERRAMIENTAS_DEV.contains(&base)
}

/// Texto para identificar el proceso: su línea de comandos o, si no hay, la ruta o el nombre.
pub fn comando_legible(cmd: &[String], ruta: &str, nombre: &str) -> String {
    if !cmd.is_empty() {
        cmd.join(" ")
    } else if !ruta.is_empty() {
        ruta.to_string()
    } else {
        nombre.to_string()
    }
}

/// Filtra, une IPv4/IPv6 (mismo puerto y PID), excluye la propia app y ordena por puerto.
pub fn preparar(candidatos: Vec<Candidato>, pid_propio: u32) -> Vec<PuertoInfo> {
    let mut vistos = HashSet::new();
    let mut lista: Vec<PuertoInfo> = candidatos
        .into_iter()
        .filter(|c| c.pid != pid_propio && es_de_desarrollo(c))
        .filter(|c| vistos.insert((c.puerto, c.pid)))
        .map(|c| PuertoInfo {
            es_dev: es_herramienta_dev(&c.nombre),
            puerto: c.puerto,
            pid: c.pid,
            nombre: c.nombre,
            comando: c.comando,
            inicio: c.inicio,
        })
        .collect();
    lista.sort_by_key(|p| (p.puerto, p.pid));
    lista
}

/// El puerto coincide con la búsqueda (número, nombre o comando; sin distinguir mayúsculas).
pub fn coincide(p: &PuertoInfo, busqueda: &str) -> bool {
    let b = busqueda.trim().to_lowercase();
    b.is_empty()
        || p.puerto.to_string().contains(&b)
        || p.nombre.to_lowercase().contains(&b)
        || p.comando.to_lowercase().contains(&b)
}
```

- [ ] **Paso 6: comprobar que pasan**

Ejecutar: `cargo test puertos`
Esperado: 11 tests en verde.

- [ ] **Paso 7: comprobaciones y commit**

```bash
cargo fmt && cargo clippy --all-targets && cargo test
git add Cargo.toml Cargo.lock src/puertos.rs src/main.rs
git commit -m "feat: añadir el filtro de puertos de desarrollo"
```

---

### Tarea 2: lectura real de los puertos

**Archivos:**
- Modificar: `src/puertos.rs`

**Interfaces:**
- Consume: `Candidato`, `PuertoInfo`, `comando_legible`, `preparar` (tarea 1).
- Produce:
  - `pub type ResultadoLectura = Result<Vec<PuertoInfo>, String>;`
  - `pub fn leer_candidatos(sys: &mut System) -> Result<Vec<Candidato>, String>`
  - `pub fn leer(sys: &mut System) -> ResultadoLectura`

- [ ] **Paso 1: escribir el test que falla**

Añadir dentro de `mod tests` de `src/puertos.rs`:

```rust
    #[test]
    fn la_lectura_encuentra_un_puerto_real_con_su_pid() {
        let servidor = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let puerto = servidor.local_addr().unwrap().port();
        let pid = std::process::id();

        let mut sys = sysinfo::System::new();
        let candidatos = leer_candidatos(&mut sys).unwrap();
        let propio = candidatos
            .iter()
            .find(|c| c.puerto == puerto && c.pid == pid)
            .expect("el puerto del test debería aparecer");
        assert!(propio.es_del_usuario);
        assert!(propio.inicio > 0);
        assert!(!propio.comando.is_empty());

        // `leer` aplica el filtro y excluye la propia app
        assert!(leer(&mut sys).unwrap().iter().all(|p| p.pid != pid));
    }
```

- [ ] **Paso 2: comprobar que falla**

Ejecutar: `cargo test la_lectura_encuentra`
Esperado: error de compilación `cannot find function leer_candidatos`.

- [ ] **Paso 3: implementar**

Añadir en `src/puertos.rs`. Primero, ampliar los `use` del principio del archivo:

```rust
use std::collections::HashSet;

use listeners::{Protocol, SocketState};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
```

Y después de `coincide`:

```rust
/// Resultado de una lectura de puertos: la lista o el mensaje de error.
pub type ResultadoLectura = Result<Vec<PuertoInfo>, String>;

/// Lee todos los sockets TCP en escucha y los completa con datos de `sysinfo`.
pub fn leer_candidatos(sys: &mut System) -> Result<Vec<Candidato>, String> {
    let sockets: Vec<listeners::Listener> = listeners::get_all()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|l| l.protocol == Protocol::TCP && l.state == SocketState::Listen)
        .collect();

    // Solo se refrescan los procesos implicados, y con los datos que hacen falta
    let propio = sysinfo::get_current_pid().ok();
    let mut pids: Vec<Pid> = sockets.iter().map(|l| Pid::from_u32(l.process.pid)).collect();
    pids.extend(propio);
    pids.sort();
    pids.dedup();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&pids),
        false,
        ProcessRefreshKind::nothing()
            .with_user(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_exe(UpdateKind::OnlyIfNotSet),
    );

    let usuario = propio
        .and_then(|p| sys.process(p))
        .and_then(|p| p.user_id())
        .cloned();

    Ok(sockets
        .iter()
        .filter_map(|l| {
            // Si el proceso terminó entre las dos lecturas, se omite
            let proceso = sys.process(Pid::from_u32(l.process.pid))?;
            let cmd: Vec<String> = proceso
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            Some(Candidato {
                puerto: l.socket.port(),
                pid: l.process.pid,
                nombre: l.process.name.clone(),
                ruta: l.process.path.clone(),
                comando: comando_legible(&cmd, &l.process.path, &l.process.name),
                inicio: proceso.start_time(),
                es_del_usuario: usuario.is_some() && proceso.user_id() == usuario.as_ref(),
            })
        })
        .collect())
}

/// Lista de puertos de desarrollo lista para mostrar.
pub fn leer(sys: &mut System) -> ResultadoLectura {
    let pid_propio = sysinfo::get_current_pid().map(|p| p.as_u32()).unwrap_or(0);
    Ok(preparar(leer_candidatos(sys)?, pid_propio))
}
```

Borrar la línea `use std::collections::HashSet;` duplicada si quedó dos veces.

- [ ] **Paso 4: comprobar que pasa**

Ejecutar: `cargo test puertos`
Esperado: 12 tests en verde.

- [ ] **Paso 5: comprobaciones y commit**

```bash
cargo fmt && cargo clippy --all-targets && cargo test
git add src/puertos.rs
git commit -m "feat: leer los puertos en escucha con listeners y sysinfo"
```

---

### Tarea 3: terminar un proceso

**Archivos:**
- Modificar: `src/puertos.rs`

**Interfaces:**
- Consume: `PuertoInfo`, `leer` (tareas 1 y 2).
- Produce:
  - `pub const ESPERA_CIERRE: Duration` (3 s)
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ResultadoCierre { Cerrado, YaTerminado, NoResponde, SinPermiso, Cambio }`
  - `pub fn terminar(p: &PuertoInfo, forzar: bool) -> ResultadoCierre` (bloquea hasta 3 s; llamarla desde un hilo aparte)

- [ ] **Paso 1: escribir los tests que fallan**

Añadir dentro de `mod tests` de `src/puertos.rs`:

```rust
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};

    const VAR_SERVIDOR: &str = "MONITOR_SERVIDOR_AUXILIAR";

    /// No es un test real: es el «modo servidor» del ejecutable de tests.
    /// Solo actúa si lo lanza `lanzar_servidor` con la variable de entorno.
    #[test]
    #[ignore]
    fn servidor_auxiliar() {
        if std::env::var(VAR_SERVIDOR).is_err() {
            return;
        }
        let servidor = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        println!("PUERTO={}", servidor.local_addr().unwrap().port());
        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    /// Lanza el propio ejecutable de tests como proceso hijo que escucha en un puerto.
    fn lanzar_servidor() -> (Child, u16) {
        let mut hijo = Command::new(std::env::current_exe().unwrap())
            .args(["puertos::tests::servidor_auxiliar", "--exact", "--ignored", "--nocapture"])
            .env(VAR_SERVIDOR, "1")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let salida = BufReader::new(hijo.stdout.take().unwrap());
        let puerto = salida
            .lines()
            .map_while(Result::ok)
            .find_map(|l| l.strip_prefix("PUERTO=").map(|p| p.parse().unwrap()))
            .expect("el servidor auxiliar no indicó su puerto");
        (hijo, puerto)
    }

    fn info_del_hijo(hijo: &Child, puerto: u16) -> PuertoInfo {
        let mut sys = sysinfo::System::new();
        leer(&mut sys)
            .unwrap()
            .into_iter()
            .find(|p| p.pid == hijo.id() && p.puerto == puerto)
            .expect("el servidor auxiliar debería aparecer en la lista")
    }

    fn puerto_libre(puerto: u16) -> bool {
        std::net::TcpListener::bind(("127.0.0.1", puerto)).is_ok()
    }

    #[test]
    fn terminar_libera_el_puerto() {
        let (mut hijo, puerto) = lanzar_servidor();
        let info = info_del_hijo(&hijo, puerto);

        assert_eq!(terminar(&info, false), ResultadoCierre::Cerrado);
        hijo.wait().unwrap();
        assert!(puerto_libre(puerto));
    }

    #[test]
    fn proceso_que_ya_termino() {
        let (mut hijo, puerto) = lanzar_servidor();
        let info = info_del_hijo(&hijo, puerto);
        hijo.kill().unwrap();
        hijo.wait().unwrap();

        assert_eq!(terminar(&info, false), ResultadoCierre::YaTerminado);
    }

    #[test]
    fn pid_reutilizado_no_se_toca() {
        let (mut hijo, puerto) = lanzar_servidor();
        let mut info = info_del_hijo(&hijo, puerto);
        info.inicio += 1; // simula otro proceso con el mismo PID

        assert_eq!(terminar(&info, false), ResultadoCierre::Cambio);
        assert!(hijo.try_wait().unwrap().is_none(), "el proceso debe seguir vivo");
        hijo.kill().unwrap();
        hijo.wait().unwrap();
    }
```

- [ ] **Paso 2: comprobar que fallan**

Ejecutar: `cargo test puertos`
Esperado: error de compilación `cannot find function terminar` y `cannot find type ResultadoCierre`.

- [ ] **Paso 3: implementar**

Ampliar los `use` del principio de `src/puertos.rs`:

```rust
use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};

use listeners::{Protocol, SocketState};
use sysinfo::{
    Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Signal, System, UpdateKind,
};
```

Y añadir después de `leer`:

```rust
/// Cuánto se espera a que un proceso se cierre tras pedírselo.
pub const ESPERA_CIERRE: Duration = Duration::from_secs(3);

/// Qué pasó al intentar terminar un proceso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoCierre {
    /// El proceso terminó.
    Cerrado,
    /// Ya no existía antes de actuar.
    YaTerminado,
    /// Sigue vivo tras la espera.
    NoResponde,
    /// El sistema no dejó enviarle la señal.
    SinPermiso,
    /// El PID ahora es de otro proceso: no se tocó.
    Cambio,
}

fn refrescar(sys: &mut System, pid: Pid) {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
}

/// El proceso de `p` sigue vivo (mismo PID y misma hora de inicio, y no es un zombi).
fn esta_vivo(sys: &mut System, p: &PuertoInfo) -> bool {
    let pid = Pid::from_u32(p.pid);
    refrescar(sys, pid);
    sys.process(pid).is_some_and(|proc_| {
        proc_.start_time() == p.inicio && proc_.status() != ProcessStatus::Zombie
    })
}

/// Termina el proceso que ocupa el puerto. Primero comprueba que el PID sigue siendo
/// el mismo proceso. Sin `forzar` envía SIGTERM (en Windows, donde no existe, cierra
/// directamente); con `forzar`, SIGKILL. Después espera hasta `ESPERA_CIERRE`.
/// Bloquea: debe llamarse desde un hilo aparte.
pub fn terminar(p: &PuertoInfo, forzar: bool) -> ResultadoCierre {
    let pid = Pid::from_u32(p.pid);
    let mut sys = System::new();
    refrescar(&mut sys, pid);

    let Some(proceso) = sys.process(pid) else {
        return ResultadoCierre::YaTerminado;
    };
    if proceso.start_time() != p.inicio {
        return ResultadoCierre::Cambio;
    }
    if proceso.status() == ProcessStatus::Zombie {
        return ResultadoCierre::YaTerminado;
    }

    let enviada = if forzar {
        proceso.kill()
    } else {
        proceso
            .kill_with(Signal::Term)
            .unwrap_or_else(|| proceso.kill())
    };
    if !enviada {
        return if esta_vivo(&mut sys, p) {
            ResultadoCierre::SinPermiso
        } else {
            ResultadoCierre::YaTerminado
        };
    }

    let limite = Instant::now() + ESPERA_CIERRE;
    while Instant::now() < limite {
        if !esta_vivo(&mut sys, p) {
            return ResultadoCierre::Cerrado;
        }
        thread::sleep(Duration::from_millis(100));
    }
    ResultadoCierre::NoResponde
}
```

- [ ] **Paso 4: comprobar que pasan**

Ejecutar: `cargo test puertos`
Esperado: 15 tests en verde y 1 ignorado (`servidor_auxiliar`).

- [ ] **Paso 5: comprobaciones y commit**

```bash
cargo fmt && cargo clippy --all-targets && cargo test
git add src/puertos.rs
git commit -m "feat: terminar el proceso de un puerto con cierre ordenado"
```

---

### Tarea 4: leer los puertos desde el hilo lector

**Archivos:**
- Modificar: `src/main.rs` (`use`, constantes, `Muestra`, `iniciar_lector`, `Monitor`, `Monitor::new`, `Monitor::recibir`, `impl eframe::App`)
- Modificar: `src/puertos.rs` (constante y función de cadencia)

**Interfaces:**
- Consume: `puertos::leer`, `puertos::ResultadoLectura`, `puertos::PuertoInfo`.
- Produce:
  - En `puertos.rs`: `pub const INTERVALO_PUERTOS: Duration` (2 s) y `pub fn toca_leer(ultima: Option<Instant>, ahora: Instant) -> bool`.
  - En `main.rs`: campos `Monitor::leer_puertos: Arc<AtomicBool>` y `Monitor::puertos: Option<puertos::ResultadoLectura>`; método `Monitor::actualizar_puertos(&mut self, nuevos: puertos::ResultadoLectura)`; campo `Muestra::puertos: Option<puertos::ResultadoLectura>`.

- [ ] **Paso 1: escribir el test que falla**

Añadir dentro de `mod tests` de `src/puertos.rs`:

```rust
    #[test]
    fn cadencia_de_lectura() {
        let ahora = Instant::now();
        assert!(toca_leer(None, ahora));
        assert!(!toca_leer(Some(ahora), ahora + Duration::from_millis(1500)));
        assert!(toca_leer(Some(ahora), ahora + INTERVALO_PUERTOS));
    }
```

- [ ] **Paso 2: comprobar que falla**

Ejecutar: `cargo test cadencia_de_lectura`
Esperado: error de compilación `cannot find function toca_leer`.

- [ ] **Paso 3: implementar la cadencia**

Añadir en `src/puertos.rs`, después de `ResultadoLectura`:

```rust
/// Cada cuánto se leen los puertos mientras la ventana completa está visible.
pub const INTERVALO_PUERTOS: Duration = Duration::from_secs(2);

/// Toca leer los puertos: nunca se han leído o ya pasó `INTERVALO_PUERTOS`.
pub fn toca_leer(ultima: Option<Instant>, ahora: Instant) -> bool {
    ultima.is_none_or(|t| ahora.duration_since(t) >= INTERVALO_PUERTOS)
}
```

Ejecutar `cargo test cadencia_de_lectura`. Esperado: PASS.

- [ ] **Paso 4: conectar el hilo lector**

En `src/main.rs`:

1. Sustituir los `use` de `std` por:

```rust
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
```

2. En `struct Muestra`, añadir al final:

```rust
    /// Puertos leídos en esta vuelta (solo cada `INTERVALO_PUERTOS` y con la ventana completa).
    puertos: Option<puertos::ResultadoLectura>,
```

3. Sustituir la firma y el bucle de `iniciar_lector` por:

```rust
/// Lanza un hilo que lee el sistema cada `INTERVALO` y envía una `Muestra` por el canal.
/// Así la interfaz nunca se bloquea esperando a `sysinfo`. Los puertos solo se leen
/// mientras `leer_puertos` está activo (ventana completa visible).
fn iniciar_lector(ctx: egui::Context, leer_puertos: Arc<AtomicBool>) -> Receiver<Muestra> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut sys = System::new_all();
        let mut ultima_lectura_puertos: Option<Instant> = None;
        loop {
```

y, dentro del bucle, justo antes de `let muestra = Muestra {`, añadir:

```rust
            let puertos = if leer_puertos.load(Ordering::Relaxed) {
                let ahora = Instant::now();
                puertos::toca_leer(ultima_lectura_puertos, ahora).then(|| {
                    ultima_lectura_puertos = Some(ahora);
                    puertos::leer(&mut sys)
                })
            } else {
                // Al volver a la ventana completa se lee enseguida
                ultima_lectura_puertos = None;
                None
            };
```

y en la construcción de `Muestra { … }` añadir el campo `puertos,` después de `procesos,`.

4. En `struct Monitor`, añadir al final:

```rust
    /// Activo mientras se ve la ventana completa: el hilo lector lee los puertos.
    leer_puertos: Arc<AtomicBool>,
    /// Última lectura de puertos (`None` hasta la primera).
    puertos: Option<puertos::ResultadoLectura>,
```

5. Sustituir `Monitor::new` por:

```rust
    fn new(ctx: egui::Context) -> Self {
        let leer_puertos = Arc::new(AtomicBool::new(true));
        Self {
            rx: iniciar_lector(ctx, leer_puertos.clone()),
            actual: None,
            hist_cpu: VecDeque::with_capacity(HISTORIAL),
            hist_ram: VecDeque::with_capacity(HISTORIAL),
            compacto: false,
            ventana_completa: None,
            pos_widget: None,
            leer_puertos,
            puertos: None,
        }
    }
```

6. Sustituir `Monitor::recibir` por:

```rust
    /// Toma las muestras nuevas que haya enviado el hilo lector (sin bloquear).
    fn recibir(&mut self) {
        let muestras: Vec<Muestra> = self.rx.try_iter().collect();
        for mut m in muestras {
            empujar(&mut self.hist_cpu, m.cpu_global);
            empujar(&mut self.hist_ram, porcentaje(m.mem_usada, m.mem_total));
            if let Some(p) = m.puertos.take() {
                self.actualizar_puertos(p);
            }
            self.actual = Some(m);
        }
    }

    /// Guarda una lectura nueva de puertos.
    fn actualizar_puertos(&mut self, nuevos: puertos::ResultadoLectura) {
        self.puertos = Some(nuevos);
    }
```

7. En `impl eframe::App for Monitor`, al principio de `fn ui`, antes de `self.recibir();`, añadir:

```rust
        self.leer_puertos.store(!self.compacto, Ordering::Relaxed);
```

- [ ] **Paso 5: comprobar**

Ejecutar: `cargo test` → todo en verde. `cargo clippy --all-targets` → sin advertencias. `cargo run --release` → la app se ve igual que antes (la interfaz aún no muestra los puertos).

- [ ] **Paso 6: commit**

```bash
cargo fmt
git add src/main.rs src/puertos.rs
git commit -m "feat: leer los puertos en segundo plano con la ventana completa"
```

---

### Tarea 5: pestañas y vista de puertos

**Archivos:**
- Modificar: `src/main.rs` (constantes, `Monitor`, `Monitor::new`, `ui_completa`, nuevas `vista_sistema` y `vista_puertos`)

**Interfaces:**
- Consume: `Monitor::puertos`, `puertos::coincide`, `puertos::PuertoInfo`, `puertos::INTERVALO_PUERTOS`.
- Produce:
  - `#[derive(Clone, Copy, PartialEq, Eq)] enum Pestana { Sistema, Puertos }`
  - `#[derive(Clone, Copy, PartialEq, Eq)] enum EstadoCierre { Cerrando, NoResponde }`
  - `struct Confirmacion { puerto: puertos::PuertoInfo, forzar: bool }` con `#[derive(Clone)]`
  - Campos de `Monitor`: `pestana: Pestana`, `busqueda: String`, `en_curso: HashMap<(u16, u32), EstadoCierre>`, `confirmacion: Option<Confirmacion>`
  - `const COLOR_PELIGRO: Color32` (rojo suave para textos de acción)
  - Métodos `fn vista_sistema(&self, ui: &mut egui::Ui)` y `fn vista_puertos(&mut self, ui: &mut egui::Ui)`

- [ ] **Paso 1: tipos y estado**

En `src/main.rs`:

1. Añadir `use std::collections::HashMap;` junto al `use std::collections::VecDeque;` (o unificar en `use std::collections::{HashMap, VecDeque};`).

2. Después de `const COLOR_RAM`, añadir:

```rust
/// Rojo suave para acciones que terminan procesos.
const COLOR_PELIGRO: Color32 = Color32::from_rgb(229, 115, 115);
```

3. Antes de `struct Monitor`, añadir:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pestana {
    Sistema,
    Puertos,
}

/// Estado de una fila de puertos mientras se intenta terminar su proceso.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EstadoCierre {
    Cerrando,
    NoResponde,
}

/// Petición de confirmación pendiente para terminar (o forzar) un proceso.
#[derive(Clone)]
struct Confirmacion {
    puerto: puertos::PuertoInfo,
    forzar: bool,
}
```

4. En `struct Monitor`, añadir al final:

```rust
    pestana: Pestana,
    busqueda: String,
    /// Filas con un cierre en marcha, por (puerto, PID).
    en_curso: HashMap<(u16, u32), EstadoCierre>,
    confirmacion: Option<Confirmacion>,
```

5. En `Monitor::new`, añadir al final de `Self { … }`:

```rust
            pestana: Pestana::Sistema,
            busqueda: String::new(),
            en_curso: HashMap::new(),
            confirmacion: None,
```

6. En `actualizar_puertos`, olvidar los cierres de filas que ya no existen. Sustituirla por:

```rust
    /// Guarda una lectura nueva de puertos y olvida los cierres de filas que ya no existen.
    fn actualizar_puertos(&mut self, nuevos: puertos::ResultadoLectura) {
        if let Ok(lista) = &nuevos {
            self.en_curso
                .retain(|clave, _| lista.iter().any(|p| (p.puerto, p.pid) == *clave));
        }
        self.puertos = Some(nuevos);
    }
```

- [ ] **Paso 2: separar la vista «Sistema»**

1. Cambiar la firma de `ui_completa` de `fn ui_completa(&self, ui: &mut egui::Ui) -> bool` a `fn ui_completa(&mut self, ui: &mut egui::Ui) -> bool`.

2. Dentro de `ui_completa`, **borrar** estas líneas del principio del `CentralPanel` (el indicador de carga se mueve a `vista_sistema`):

```rust
            // Hasta que llegue la primera lectura
            let Some(m) = &self.actual else {
                ui.centered_and_justified(|ui| ui.spinner());
                return;
            };
```

3. Crear un método nuevo en el mismo `impl Monitor`:

```rust
    /// Pestaña «Sistema»: CPU, memoria y procesos (la vista de siempre).
    fn vista_sistema(&self, ui: &mut egui::Ui) {
        // Hasta que llegue la primera lectura
        let Some(m) = &self.actual else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };
        // ↓ aquí va el bloque movido (ver punto 4)
    }
```

4. **Mover** a `vista_sistema`, en el lugar del comentario, **sin cambios** todo el código de `ui_completa` que va desde la línea `// ── CPU ────…` hasta el final de la sección `// ── Top procesos ──…` (el `seccion(ui, |ui| { … TableBuilder … });` de procesos, incluido). En `ui_completa` quedan el encabezado (título, versión, encendido, icono PiP y nombre del equipo) y el `ui.add_space(8.0);` que lo sigue.

5. En `ui_completa`, justo después de ese `ui.add_space(8.0);` del encabezado, añadir las pestañas y el reparto:

```rust
                    // ── Pestañas ───────────────────────────────
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.pestana, Pestana::Sistema, "Sistema");
                        let titulo = match &self.puertos {
                            Some(Ok(lista)) => format!("Puertos · {}", lista.len()),
                            _ => "Puertos".to_string(),
                        };
                        ui.selectable_value(&mut self.pestana, Pestana::Puertos, titulo);
                    });
                    ui.add_space(8.0);

                    match self.pestana {
                        Pestana::Sistema => self.vista_sistema(ui),
                        Pestana::Puertos => self.vista_puertos(ui),
                    }
```

- [ ] **Paso 3: la vista «Puertos»**

Añadir en el mismo `impl Monitor`:

```rust
    /// Pestaña «Puertos»: buscador y tabla de puertos de desarrollo.
    fn vista_puertos(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.busqueda)
                .hint_text("🔍 Buscar puerto o proceso…")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(6.0);

        let lista = match &self.puertos {
            None => {
                ui.spinner();
                return;
            }
            Some(Err(e)) => {
                ui.colored_label(COLOR_PELIGRO, format!("No se pudieron leer los puertos: {e}"));
                return;
            }
            Some(Ok(lista)) => lista,
        };
        if lista.is_empty() {
            ui.label("No hay puertos de desarrollo en uso.");
            ui.label(
                RichText::new(
                    "¿No ves tu puerto? Puede estar ocupado por un servicio del sistema o de otro usuario.",
                )
                .weak(),
            );
            return;
        }
        let visibles: Vec<&puertos::PuertoInfo> = lista
            .iter()
            .filter(|p| puertos::coincide(p, &self.busqueda))
            .collect();
        if visibles.is_empty() {
            ui.label(format!(
                "Ningún puerto coincide con «{}».",
                self.busqueda.trim()
            ));
            return;
        }

        let mut pedir: Option<Confirmacion> = None;
        TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(56.0))
            .column(Column::remainder().clip(true))
            .column(Column::exact(104.0))
            .header(20.0, |mut fila| {
                fila.col(|ui| {
                    ui.strong("Puerto");
                });
                fila.col(|ui| {
                    ui.strong("Proceso");
                });
                fila.col(|ui| {
                    ui.strong("Acción");
                });
            })
            .body(|mut cuerpo| {
                for p in &visibles {
                    cuerpo.row(40.0, |mut fila| {
                        fila.col(|ui| {
                            ui.label(RichText::new(p.puerto.to_string()).strong().size(16.0));
                        });
                        fila.col(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(RichText::new(&p.nombre).strong())
                                            .truncate(),
                                    );
                                    if p.es_dev {
                                        ui.label(
                                            RichText::new(" dev ")
                                                .small()
                                                .color(TEXTO_OSCURO)
                                                .background_color(COLOR_CPU),
                                        );
                                    }
                                });
                                ui.add(
                                    egui::Label::new(RichText::new(&p.comando).small().weak())
                                        .truncate(),
                                )
                                .on_hover_text(format!("{}\nPID {}", p.comando, p.pid));
                            });
                        });
                        fila.col(|ui| match self.en_curso.get(&(p.puerto, p.pid)) {
                            Some(EstadoCierre::Cerrando) => {
                                ui.spinner();
                                ui.label("Cerrando…");
                            }
                            Some(EstadoCierre::NoResponde) => {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("No responde").small().weak());
                                    if ui
                                        .button(RichText::new("Forzar cierre").color(COLOR_PELIGRO))
                                        .clicked()
                                    {
                                        pedir = Some(Confirmacion {
                                            puerto: (*p).clone(),
                                            forzar: true,
                                        });
                                    }
                                });
                            }
                            None => {
                                if ui
                                    .button(RichText::new("Terminar").color(COLOR_PELIGRO))
                                    .clicked()
                                {
                                    pedir = Some(Confirmacion {
                                        puerto: (*p).clone(),
                                        forzar: false,
                                    });
                                }
                            }
                        });
                    });
                }
            });
        if pedir.is_some() {
            self.confirmacion = pedir;
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Se actualiza cada {} s",
                puertos::INTERVALO_PUERTOS.as_secs()
            ))
            .small()
            .weak(),
        );
    }
```

- [ ] **Paso 4: comprobar**

Ejecutar `cargo fmt && cargo clippy --all-targets && cargo test` → sin advertencias y todo en verde.

Ejecutar `cargo run --release` y comprobar:
- La pestaña «Sistema» se ve igual que antes.
- Con `python3 -m http.server 4200` corriendo en otra terminal, la pestaña dice «Puertos · N» y lista el 4200 con `python3` [dev] y su comando.
- El buscador filtra por «4200» y por «http».
- El botón «Terminar» todavía no hace nada (lo conecta la tarea 6).

- [ ] **Paso 5: commit**

```bash
git add src/main.rs
git commit -m "feat: añadir las pestañas y la vista de puertos"
```

---

### Tarea 6: confirmación, cierre en segundo plano y avisos

**Archivos:**
- Modificar: `src/puertos.rs` (mensajes de resultado)
- Modificar: `src/main.rs` (quitar el `allow(dead_code)`, estado del aviso y del canal, diálogo modal, lanzar y procesar cierres)

**Interfaces:**
- Consume: `puertos::terminar`, `puertos::ResultadoCierre`, `Confirmacion`, `EstadoCierre`, `Monitor::en_curso`, `Monitor::confirmacion` (tareas 3 y 5).
- Produce:
  - En `puertos.rs`: `pub fn mensaje_resultado(r: ResultadoCierre, puerto: u16) -> (String, bool)` (texto y si es un éxito) y `impl ResultadoCierre { pub fn quita_fila(self) -> bool }`.
  - En `main.rs`: `struct Aviso { texto: String, ok: bool, hasta: Instant }` y los métodos `dialogo_confirmacion`, `lanzar_cierre`, `procesar_cierre`.

- [ ] **Paso 1: escribir el test que falla**

Añadir dentro de `mod tests` de `src/puertos.rs`:

```rust
    #[test]
    fn mensajes_de_resultado() {
        assert_eq!(
            mensaje_resultado(ResultadoCierre::Cerrado, 4200),
            ("✓ Puerto 4200 liberado".to_string(), true)
        );
        assert_eq!(
            mensaje_resultado(ResultadoCierre::YaTerminado, 4200),
            ("El proceso ya había terminado.".to_string(), true)
        );
        assert_eq!(
            mensaje_resultado(ResultadoCierre::SinPermiso, 4200),
            ("No tienes permiso para terminar este proceso.".to_string(), false)
        );
        assert_eq!(
            mensaje_resultado(ResultadoCierre::Cambio, 4200),
            ("El proceso ya no existe o cambió. La lista se ha actualizado.".to_string(), false)
        );
        assert!(!mensaje_resultado(ResultadoCierre::NoResponde, 4200).1);
    }

    #[test]
    fn que_resultados_quitan_la_fila() {
        assert!(ResultadoCierre::Cerrado.quita_fila());
        assert!(ResultadoCierre::YaTerminado.quita_fila());
        assert!(ResultadoCierre::Cambio.quita_fila());
        assert!(!ResultadoCierre::NoResponde.quita_fila());
        assert!(!ResultadoCierre::SinPermiso.quita_fila());
    }
```

- [ ] **Paso 2: comprobar que falla**

Ejecutar: `cargo test mensajes_de_resultado`
Esperado: error de compilación `cannot find function mensaje_resultado`.

- [ ] **Paso 3: implementar los mensajes**

Añadir en `src/puertos.rs`, después de `terminar`:

```rust
impl ResultadoCierre {
    /// La fila debe desaparecer de la lista (el puerto quedó libre o el dato ya no vale).
    pub fn quita_fila(self) -> bool {
        matches!(
            self,
            ResultadoCierre::Cerrado | ResultadoCierre::YaTerminado | ResultadoCierre::Cambio
        )
    }
}

/// Texto del aviso tras intentar terminar un proceso, y si es un éxito.
pub fn mensaje_resultado(r: ResultadoCierre, puerto: u16) -> (String, bool) {
    match r {
        ResultadoCierre::Cerrado => (format!("✓ Puerto {puerto} liberado"), true),
        ResultadoCierre::YaTerminado => ("El proceso ya había terminado.".to_string(), true),
        ResultadoCierre::SinPermiso => (
            "No tienes permiso para terminar este proceso.".to_string(),
            false,
        ),
        ResultadoCierre::Cambio => (
            "El proceso ya no existe o cambió. La lista se ha actualizado.".to_string(),
            false,
        ),
        // Solo llega aquí tras «Forzar cierre»
        ResultadoCierre::NoResponde => (
            "El proceso no se cerró. Puede que necesites permisos de administrador.".to_string(),
            false,
        ),
    }
}
```

Ejecutar: `cargo test puertos`. Esperado: todo en verde.

- [ ] **Paso 4: estado del aviso y del canal**

En `src/main.rs`:

1. **Borrar** las dos líneas añadidas en la tarea 1 sobre `mod puertos;` (el comentario y `#[cfg_attr(not(test), allow(dead_code))]`), dejando solo `mod puertos;`.

2. Cambiar `use std::sync::mpsc::{self, Receiver};` por `use std::sync::mpsc::{self, Receiver, Sender};`.

3. Después de `const COLOR_PELIGRO`, añadir:

```rust
/// Rojo intenso para el botón que confirma el cierre (texto blanco encima).
const COLOR_PELIGRO_FUERTE: Color32 = Color32::from_rgb(198, 40, 40);
/// Cuánto se ve el aviso tras terminar un proceso.
const DURACION_AVISO: Duration = Duration::from_secs(4);
```

4. Después de `struct Confirmacion`, añadir:

```rust
/// Aviso temporal sobre la lista de puertos.
#[derive(Clone)]
struct Aviso {
    texto: String,
    ok: bool,
    hasta: Instant,
}

/// Lo que devuelve el hilo de cierre: puerto, si se forzó y resultado.
type RespuestaCierre = (puertos::PuertoInfo, bool, puertos::ResultadoCierre);
```

5. En `struct Monitor`, añadir al final:

```rust
    aviso: Option<Aviso>,
    tx_cierre: Sender<RespuestaCierre>,
    rx_cierre: Receiver<RespuestaCierre>,
```

6. En `Monitor::new`, crear el canal antes de `Self {` con `let (tx_cierre, rx_cierre) = mpsc::channel();` y añadir al final de `Self { … }`:

```rust
            aviso: None,
            tx_cierre,
            rx_cierre,
```

- [ ] **Paso 5: lanzar y procesar cierres**

Añadir en `impl Monitor` (el primero, junto a `recibir`):

```rust
    /// Marca la fila como «Cerrando…» y termina el proceso en un hilo aparte.
    fn lanzar_cierre(&mut self, p: puertos::PuertoInfo, forzar: bool, ctx: &egui::Context) {
        self.en_curso
            .insert((p.puerto, p.pid), EstadoCierre::Cerrando);
        let tx = self.tx_cierre.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let r = puertos::terminar(&p, forzar);
            let _ = tx.send((p, forzar, r));
            ctx.request_repaint();
        });
    }

    /// Aplica el resultado de un cierre: estado de la fila, lista y aviso.
    fn procesar_cierre(&mut self, p: puertos::PuertoInfo, forzar: bool, r: puertos::ResultadoCierre) {
        let clave = (p.puerto, p.pid);
        if r == puertos::ResultadoCierre::NoResponde && !forzar {
            self.en_curso.insert(clave, EstadoCierre::NoResponde);
            return;
        }
        self.en_curso.remove(&clave);
        if r.quita_fila()
            && let Some(Ok(lista)) = &mut self.puertos
        {
            lista.retain(|x| (x.puerto, x.pid) != clave);
        }
        let (texto, ok) = puertos::mensaje_resultado(r, p.puerto);
        self.aviso = Some(Aviso {
            texto,
            ok,
            hasta: Instant::now() + DURACION_AVISO,
        });
    }
```

Y al final de `recibir`, después del bucle `for mut m in muestras { … }`, añadir:

```rust
        let cierres: Vec<RespuestaCierre> = self.rx_cierre.try_iter().collect();
        for (p, forzar, r) in cierres {
            self.procesar_cierre(p, forzar, r);
        }
```

- [ ] **Paso 6: mostrar el aviso**

Al principio de `vista_puertos`, antes del `TextEdit`, añadir:

```rust
        // Aviso temporal tras terminar un proceso
        let ahora = Instant::now();
        match self.aviso.clone() {
            Some(aviso) if aviso.hasta > ahora => {
                let color = if aviso.ok {
                    color_carga(0.0)
                } else {
                    COLOR_PELIGRO
                };
                ui.label(RichText::new(&aviso.texto).color(color).strong());
                ui.ctx().request_repaint_after(aviso.hasta - ahora);
            }
            Some(_) => self.aviso = None,
            None => {}
        }
```

- [ ] **Paso 7: el diálogo de confirmación**

Añadir en el segundo `impl Monitor` (el de la interfaz):

```rust
    /// Diálogo modal para confirmar que se termina (o se fuerza) un proceso.
    fn dialogo_confirmacion(&mut self, ctx: &egui::Context) {
        let Some(conf) = self.confirmacion.clone() else {
            return;
        };
        let p = &conf.puerto;
        let mut decision: Option<bool> = None; // Some(true) = terminar, Some(false) = cancelar

        let modal = egui::Modal::new(egui::Id::new("confirmar_cierre")).show(ctx, |ui| {
            ui.set_max_width(340.0);
            if conf.forzar {
                ui.heading(format!("Forzar el cierre del puerto {}", p.puerto));
                ui.label("El proceso no respondió. ¿Forzar el cierre? No podrá guardar nada.");
            } else {
                ui.heading(format!("¿Terminar el proceso del puerto {}?", p.puerto));
            }
            ui.add_space(6.0);
            ui.strong(format!("{} · PID {}", p.nombre, p.pid));
            // Línea de comandos completa, con ajuste de línea
            ui.label(RichText::new(&p.comando).small().weak());
            if !conf.forzar {
                ui.add_space(6.0);
                ui.label(if cfg!(windows) {
                    "En Windows el cierre es inmediato. Si tiene trabajo sin guardar, se perderá."
                } else {
                    "Si tiene trabajo sin guardar, se perderá."
                });
            }
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let accion = if conf.forzar { "Forzar cierre" } else { "Terminar" };
                if ui
                    .add(
                        egui::Button::new(RichText::new(accion).color(Color32::WHITE))
                            .fill(COLOR_PELIGRO_FUERTE),
                    )
                    .clicked()
                {
                    decision = Some(true);
                }
                let cancelar = ui.button("Cancelar");
                if cancelar.clicked() {
                    decision = Some(false);
                }
                // «Cancelar» es la opción por defecto: Intro la activa
                if ui.memory(|m| m.focused().is_none()) {
                    cancelar.request_focus();
                }
            });
        });
        // Esc o clic fuera del diálogo: cancelar
        if decision.is_none() && modal.should_close() {
            decision = Some(false);
        }

        match decision {
            Some(true) => {
                self.confirmacion = None;
                self.lanzar_cierre(conf.puerto, conf.forzar, ctx);
            }
            Some(false) => self.confirmacion = None,
            None => {}
        }
    }
```

Y en `impl eframe::App for Monitor`, en `fn ui`, sustituir:

```rust
        let cambiar = if self.compacto {
            self.ui_compacta(ui)
        } else {
            self.ui_completa(ui)
        };
```

por:

```rust
        let cambiar = if self.compacto {
            self.ui_compacta(ui)
        } else {
            let cambiar = self.ui_completa(ui);
            self.dialogo_confirmacion(ui.ctx());
            cambiar
        };
```

- [ ] **Paso 8: comprobar**

Ejecutar `cargo fmt && cargo clippy --all-targets && cargo test` → sin advertencias y todo en verde.

Ejecutar `cargo run --release` con `python3 -m http.server 4200` en otra terminal:
- «Terminar» abre el diálogo con la línea de comandos completa; Esc y «Cancelar» lo cierran sin hacer nada.
- «Terminar» en el diálogo: la fila muestra «Cerrando…», después desaparece y se ve «✓ Puerto 4200 liberado» durante unos segundos.

- [ ] **Paso 9: commit**

```bash
git add src/main.rs src/puertos.rs
git commit -m "feat: terminar procesos desde la pestaña de puertos"
```

---

### Tarea 7: documentación y prueba manual

**Archivos:**
- Modificar: `CHANGELOG.md` (sección `[Sin publicar]`)
- Modificar: `README.md` (sección `## Uso`, después de la nota de Wayland del modo compacto)
- Modificar: `README.en.md` (sección `## Usage`, después de la nota de Wayland del modo compacto)
- Modificar: `docs/diseño/2026-09-25-puertos.md` (apartado «Cuándo se leen»)

- [ ] **Paso 1: CHANGELOG**

En `CHANGELOG.md`, sustituir `## [Sin publicar]\n` por:

```markdown
## [Sin publicar]

### Añadido
- Pestaña «Puertos»: lista los puertos TCP de desarrollo en escucha (de tu usuario, 1024 o mayores y fuera de las carpetas del sistema) con el proceso que ocupa cada uno, su línea de comandos y la etiqueta [dev] para herramientas conocidas. Tiene buscador y permite terminar el proceso con confirmación: primero se le pide que se cierre y, si no responde en 3 segundos, se puede forzar.
```

- [ ] **Paso 2: README en español**

En `README.md`, después de la línea `> En Linux con Wayland, algunos gestores de ventanas ignoran la opción "siempre visible".`, añadir:

```markdown

### Puertos
La pestaña **Puertos** muestra qué proceso ocupa cada puerto de desarrollo, por ejemplo el 4200 de un `ng serve` que no se cerró bien, y permite terminarlo.

- Solo aparecen puertos TCP en escucha de **tu usuario**, del **1024 en adelante** y de programas que **no son del sistema**. Los servicios del sistema, como AirPlay en el 5000, no se muestran.
- Cada fila enseña el puerto, el proceso (con la etiqueta **[dev]** si es `node`, `python`, `java`…) y su línea de comandos. Pasa el ratón por encima para verla completa con el PID.
- El buscador filtra por número de puerto, nombre o comando.
- **Terminar** pide confirmación y después solicita al proceso que se cierre. Si no responde en 3 segundos, aparece **Forzar cierre**. En Windows el cierre es inmediato.
```

- [ ] **Paso 3: README en inglés**

En `README.en.md`, después de la línea `Some Linux window managers running Wayland may ignore the always-on-top setting.`, añadir:

```markdown

### Ports

The **Ports** tab shows which process is holding each development port, such as port 4200 from an `ng serve` that did not shut down cleanly, and lets you terminate it.

- Only listening TCP ports owned by **your user**, numbered **1024 or higher** and opened by **non-system programs** are listed. System services, such as AirPlay on port 5000, are hidden.
- Each row shows the port, the process (tagged **[dev]** for `node`, `python`, `java` and similar tools) and its command line. Hover over it to see the full command and the PID.
- The search box filters by port number, process name or command.
- **Terminar** (Terminate) asks for confirmation and then requests the process to exit. If it does not respond within 3 seconds, **Forzar cierre** (Force quit) appears. On Windows the process is terminated immediately.
```

- [ ] **Paso 4: corregir el diseño**

En `docs/diseño/2026-09-25-puertos.md`, en el apartado `### Cuándo se leen`, sustituir las dos viñetas por:

```markdown
- **Mientras la ventana completa está visible (en cualquier pestaña)** y **cada 2 segundos**, para que «Puertos · N» esté al día sin entrar en la pestaña. En modo compacto no se leen.
- Lo hace el hilo lector existente, que recibe por una bandera compartida si la ventana completa está visible. La interfaz nunca se bloquea.
```

- [ ] **Paso 5: commit**

```bash
git add CHANGELOG.md README.md README.en.md "docs/diseño/2026-09-25-puertos.md"
git commit -m "docs: documentar la pestaña de puertos"
```

- [ ] **Paso 6: prueba manual con el usuario (macOS)**

Pedir al usuario que ejecute `cargo run --release` y compruebe:

1. `python3 -m http.server 4200` en otra terminal: aparece con [dev] y se encuentra con el buscador.
2. Terminar: confirmación, «Cerrando…», «✓ Puerto 4200 liberado». El servidor vuelve a arrancar en el mismo puerto.
3. Un proceso que ignora SIGTERM:
   `python3 -c "import signal,http.server as h; signal.signal(signal.SIGTERM, signal.SIG_IGN); h.HTTPServer(('',4300), h.SimpleHTTPRequestHandler).serve_forever()"`
   Terminar → «No responde» → «Forzar cierre» → confirmación → «✓ Puerto 4300 liberado».
4. AirPlay (5000 y 7000) no aparece.
5. La pestaña «Sistema» y el widget compacto funcionan igual.

- [ ] **Paso 7: subir y abrir el PR**

Solo cuando el usuario confirme la prueba manual:

```bash
git push -u origin feature/puertos
gh pr create --base develop --head feature/puertos --title "feat: añadir la pestaña de puertos" --body "…"
```

El cuerpo del PR resume la funcionalidad, las comprobaciones (`cargo test`, `fmt`, `clippy`, prueba manual) y la corrección al diseño sobre la cadencia de lectura. Sin líneas de atribución. El PR lo fusiona el usuario.
