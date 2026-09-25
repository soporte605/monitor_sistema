//! Pestaña «Puertos»: qué proceso ocupa cada puerto de desarrollo y cómo terminarlo.

use std::collections::HashSet;

use listeners::{Protocol, SocketState};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

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
    let mut pids: Vec<Pid> = sockets
        .iter()
        .map(|l| Pid::from_u32(l.process.pid))
        .collect();
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
        assert!(es_de_desarrollo(&candidato(
            4200,
            10,
            "node",
            "/usr/local/bin/node"
        )));
    }

    #[test]
    fn proceso_de_otro_usuario_no_entra() {
        let mut c = candidato(4200, 10, "node", "/usr/local/bin/node");
        c.es_del_usuario = false;
        assert!(!es_de_desarrollo(&c));
    }

    #[test]
    fn puerto_menor_que_1024_no_entra() {
        assert!(!es_de_desarrollo(&candidato(
            80,
            10,
            "node",
            "/usr/local/bin/node"
        )));
        assert!(es_de_desarrollo(&candidato(
            1024,
            10,
            "node",
            "/usr/local/bin/node"
        )));
    }

    #[test]
    fn ejecutables_del_sistema_no_entran() {
        assert!(en_carpeta_del_sistema(
            "/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter"
        ));
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
        for n in [
            "node",
            "node.exe",
            "NODE",
            "python3",
            "python3.12",
            "java",
            "deno",
            "bun",
            "dotnet",
            "ruby",
            "php",
            "go",
        ] {
            assert!(es_herramienta_dev(n), "{n}");
        }
        for n in ["Safari", "Spotify", "ControlCenter"] {
            assert!(!es_herramienta_dev(n), "{n}");
        }
    }

    #[test]
    fn comando_vacio_usa_la_ruta_o_el_nombre() {
        let args = vec!["node".to_string(), "server.js".to_string()];
        assert_eq!(
            comando_legible(&args, "/usr/local/bin/node", "node"),
            "node server.js"
        );
        assert_eq!(
            comando_legible(&[], "/usr/local/bin/node", "node"),
            "/usr/local/bin/node"
        );
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
}
