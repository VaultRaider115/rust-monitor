use std::io::{Read, Write};
use std::num::NonZeroU16;
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};

/// Lo que la terminal le cuenta a la interfaz.
#[derive(Debug, Clone)]
pub enum EventoTerminal {
    /// Pantalla ya interpretada (texto plano: el emulador descarta los colores).
    Pantalla(String),
    /// El proceso `ssh` terminó o falló.
    Cerrada(String),
}

/// Sesión ssh sobre una PTY real: `ssh` pregunta la contraseña dentro de la
/// terminal, igual que en una consola.
pub struct TerminalSesion {
    escritor: Arc<Mutex<Box<dyn Write + Send>>>,
    hijo: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}

impl TerminalSesion {
    pub fn iniciar(
        usuario: &str,
        host: &str,
        puerto: u16,
        columnas: u16,
        filas: u16,
        mut al_evento: impl FnMut(EventoTerminal) + Send + 'static,
    ) -> Result<Self, String> {
        let par = native_pty_system()
            .openpty(PtySize {
                rows: filas,
                cols: columnas,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| format!("No se pudo abrir la pseudo-terminal: {error}"))?;

        // `-tt` fuerza PTY remoto aunque el ssh local ya corra dentro de una.
        let mut comando = CommandBuilder::new("ssh");
        comando.arg("-tt");
        comando.arg("-p");
        comando.arg(puerto.to_string());
        comando.arg(format!("{usuario}@{host}"));
        comando.env("TERM", "xterm-256color");

        let hijo = par
            .slave
            .spawn_command(comando)
            .map_err(|error| format!("No se pudo ejecutar ssh: {error}"))?;
        drop(par.slave);

        let mut lector = par
            .master
            .try_clone_reader()
            .map_err(|error| format!("No se pudo leer la terminal: {error}"))?;
        let escritor = par
            .master
            .take_writer()
            .map_err(|error| format!("No se pudo escribir en la terminal: {error}"))?;

        // La rejilla es fija: la PTY no se redimensiona con la ventana.
        // shortcut: sin resize ni scrollback, añadir cuando la ventana se pueda cambiar de tamaño.
        let filas_utiles = NonZeroU16::new(filas).unwrap_or(NonZeroU16::MIN);
        let columnas_utiles = NonZeroU16::new(columnas).unwrap_or(NonZeroU16::MIN);
        thread::spawn(move || {
            let mut emulador = vt100::Parser::new(filas_utiles, columnas_utiles, 0);
            let mut bufer = [0u8; 8192];
            let motivo = loop {
                match lector.read(&mut bufer) {
                    Ok(0) => break "La sesión SSH terminó.".to_string(),
                    Ok(leidos) => {
                        emulador.process(&bufer[..leidos]);
                        al_evento(EventoTerminal::Pantalla(emulador.screen().contents()));
                    }
                    Err(error) => break format!("Error leyendo la terminal: {error}"),
                }
            };
            al_evento(EventoTerminal::Cerrada(motivo));
        });

        Ok(Self {
            escritor: Arc::new(Mutex::new(escritor)),
            hijo: Arc::new(Mutex::new(hijo)),
        })
    }

    pub fn escribir(&self, texto: &str) {
        if let Ok(mut escritor) = self.escritor.lock() {
            let _ = escritor.write_all(texto.as_bytes());
            let _ = escritor.flush();
        }
    }
}

impl Drop for TerminalSesion {
    fn drop(&mut self) {
        if let Ok(mut hijo) = self.hijo.lock() {
            let _ = hijo.kill();
        }
    }
}

/// Acepta `usuario@host` o `usuario@host:puerto`.
///
/// shortcut: sin IPv6 entre corchetes ni alias de `~/.ssh/config`, ampliar si hace falta.
pub fn parse_destino(destino: &str) -> Result<(String, String, u16), String> {
    let formato = "Formato esperado: usuario@host[:puerto]";

    let (resto, puerto) = match destino.rsplit_once(':') {
        Some((resto, puerto)) => {
            let puerto = puerto
                .parse::<u16>()
                .map_err(|_| format!("Puerto inválido en «{destino}». {formato}"))?;
            (resto, puerto)
        }
        None => (destino, 22),
    };

    let (usuario, host) = resto
        .split_once('@')
        .ok_or_else(|| format!("Falta el usuario en «{destino}». {formato}"))?;

    if usuario.is_empty() || host.is_empty() || host.contains(':') {
        return Err(format!("Destino inválido en «{destino}». {formato}"));
    }

    Ok((usuario.to_string(), host.to_string(), puerto))
}

#[cfg(test)]
mod tests {
    use super::parse_destino;

    #[test]
    fn usa_el_puerto_22_por_defecto() {
        assert_eq!(
            parse_destino("root@servidor").unwrap(),
            ("root".to_string(), "servidor".to_string(), 22)
        );
    }

    #[test]
    fn acepta_puerto_explicito() {
        assert_eq!(
            parse_destino("deploy@10.0.0.5:2222").unwrap(),
            ("deploy".to_string(), "10.0.0.5".to_string(), 2222)
        );
    }

    #[test]
    fn rechaza_destinos_sin_usuario_o_con_puerto_malo() {
        assert!(parse_destino("servidor").is_err());
        assert!(parse_destino("root@servidor:no-es-puerto").is_err());
    }
}
