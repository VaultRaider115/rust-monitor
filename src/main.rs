mod admin;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use admin::conexion::{abrir, pedir_metricas};
use admin::ssh::{EventoTerminal, TerminalSesion, parse_destino};
use admin::system::metrics_response::MetricsResponse;
use slint::{ComponentHandle, ModelRc, VecModel, Weak};
use tokio::time::sleep;

slint::include_modules!();

const INTERVALO_METRICAS: Duration = Duration::from_secs(2);
/// Rejilla de la PTY SSH (caracteres), fija mientras la ventana no cambie de tamaño.
const TERMINAL_COLUMNAS: u16 = 60;
const TERMINAL_FILAS: u16 = 30;

type Terminal = Rc<RefCell<Option<TerminalSesion>>>;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let runtime = Arc::new(tokio::runtime::Runtime::new().expect("no se pudo crear el runtime de tokio"));
    let generacion = Arc::new(AtomicU64::new(0));
    let terminal: Terminal = Rc::new(RefCell::new(None));

    conectar_metricas(&app, runtime.clone(), generacion.clone());
    desconectar(&app, generacion, terminal.clone());
    conectar_ssh(&app, terminal.clone());
    enviar_comandos(&app, terminal);

    let resultado = app.run();
    // Al salir, el runtime se lleva las tareas de WebSocket y el Drop de la sesión mata a ssh.
    drop(runtime);
    resultado
}

/// Cada conexión tiene un número; al cambiar de servidor las tareas viejas mueren solas.
fn conectar_metricas(
    app: &AppWindow,
    runtime: Arc<tokio::runtime::Runtime>,
    generacion: Arc<AtomicU64>,
) {
    let debil = app.as_weak();
    app.on_connect(move |nombre, url| {
        let Some(app) = debil.upgrade() else { return };
        let mia = generacion.fetch_add(1, Ordering::SeqCst) + 1;

        app.set_servidor(nombre.clone());
        app.set_show_stats(true);
        app.set_en_linea(false);
        app.set_estado("Conectando…".into());
        app.set_saludo("".into());
        app.set_error_message("".into());

        let debil = debil.clone();
        let generacion = generacion.clone();
        let url = url.to_string();

        runtime.spawn(async move {
            loop {
                if generacion.load(Ordering::SeqCst) != mia {
                    return;
                }

                match abrir(&url).await {
                    Ok((mut sesion, saludo)) => {
                        if !en_ui(&debil, move |app| {
                            app.set_saludo(saludo.into());
                            app.set_en_linea(true);
                            app.set_estado("En línea".into());
                            app.set_error_message("".into());
                        }) {
                            return;
                        }

                        loop {
                            if generacion.load(Ordering::SeqCst) != mia {
                                return;
                            }

                            match pedir_metricas(&mut sesion).await {
                                Ok(texto) => match serde_json::from_str::<MetricsResponse>(&texto) {
                                    Ok(metricas) => {
                                        if !en_ui(&debil, move |app| aplicar_metricas(app, &metricas))
                                        {
                                            return;
                                        }
                                    }
                                    // El servidor responde texto plano si aún no hay muestra.
                                    Err(_) => {
                                        if !en_ui(&debil, move |app| {
                                            app.set_error_message(texto.into());
                                        }) {
                                            return;
                                        }
                                    }
                                },
                                Err(error) => {
                                    if !en_ui(&debil, move |app| {
                                        app.set_en_linea(false);
                                        app.set_estado("Reconectando…".into());
                                        app.set_error_message(error.into());
                                    }) {
                                        return;
                                    }
                                    break;
                                }
                            }

                            sleep(INTERVALO_METRICAS).await;
                        }
                    }
                    Err(error) => {
                        if !en_ui(&debil, move |app| {
                            app.set_en_linea(false);
                            app.set_estado("Reconectando…".into());
                            app.set_error_message(error.into());
                        }) {
                            return;
                        }
                    }
                }

                sleep(INTERVALO_METRICAS).await;
            }
        });
    });
}

fn desconectar(app: &AppWindow, generacion: Arc<AtomicU64>, terminal: Terminal) {
    let debil = app.as_weak();
    app.on_desconectar(move || {
        generacion.fetch_add(1, Ordering::SeqCst);
        *terminal.borrow_mut() = None;

        if let Some(app) = debil.upgrade() {
            app.set_show_stats(false);
            app.set_en_linea(false);
            app.set_estado("Sin conexión".into());
            app.set_saludo("".into());
            app.set_error_message("".into());
            app.set_terminal_texto("".into());
            app.set_ssh_conectado(false);
            app.set_ssh_estado("desconectada".into());
        }
    });
}

fn conectar_ssh(app: &AppWindow, terminal: Terminal) {
    let debil = app.as_weak();
    app.on_ssh_conectar(move |destino| {
        let Some(app) = debil.upgrade() else { return };

        let (usuario, host, puerto) = match parse_destino(&destino) {
            Ok(datos) => datos,
            Err(mensaje) => {
                app.set_terminal_texto(format!("{mensaje}\n").into());
                return;
            }
        };

        // Cierra la sesión anterior antes de abrir otra.
        *terminal.borrow_mut() = None;
        app.set_ssh_conectado(true);
        app.set_ssh_estado(format!("{usuario}@{host}:{puerto}").into());
        app.set_terminal_texto(format!("Conectando a {usuario}@{host}:{puerto}…\n").into());

        let al_evento = {
            let debil = debil.clone();
            move |evento: EventoTerminal| {
                let debil = debil.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = debil.upgrade() else { return };
                    match evento {
                        EventoTerminal::Pantalla(texto) => app.set_terminal_texto(texto.into()),
                        EventoTerminal::Cerrada(motivo) => {
                            app.set_ssh_conectado(false);
                            app.set_ssh_estado("desconectada".into());
                            let pantalla = app.get_terminal_texto();
                            app.set_terminal_texto(format!("{pantalla}\n[{motivo}]").into());
                        }
                    }
                });
            }
        };

        match TerminalSesion::iniciar(
            &usuario,
            &host,
            puerto,
            TERMINAL_COLUMNAS,
            TERMINAL_FILAS,
            al_evento,
        ) {
            Ok(sesion) => *terminal.borrow_mut() = Some(sesion),
            Err(error) => {
                app.set_ssh_conectado(false);
                app.set_ssh_estado("error".into());
                app.set_terminal_texto(format!("{error}\n").into());
            }
        }
    });
}

fn enviar_comandos(app: &AppWindow, terminal: Terminal) {
    let envio = terminal.clone();
    app.on_ssh_enviar(move |comando| {
        if let Some(sesion) = envio.borrow().as_ref() {
            // La tecla Enter de una terminal manda CR, no LF.
            sesion.escribir(&format!("{comando}\r"));
        }
    });

    app.on_ssh_interrumpir(move || {
        if let Some(sesion) = terminal.borrow().as_ref() {
            sesion.escribir("\u{3}");
        }
    });
}

fn aplicar_metricas(app: &AppWindow, metricas: &MetricsResponse) {
    let cpu = metricas.cpu().usage();
    app.set_cpu_porcentaje(cpu as f32);
    app.set_cpu_valor(format!("{cpu:.1} %").into());

    // El servidor reporta RAM en MiB; aquí se muestra en GiB.
    let total = metricas.ram().total() / 1024.0;
    let libre = metricas.ram().available() / 1024.0;
    let usada = (total - libre).max(0.0);

    app.set_ram_porcentaje(porcentaje(usada, total));
    app.set_ram_valor(format!("{usada:.1} / {total:.1} GiB").into());
    app.set_ram_detalle(
        format!(
            "disponible {libre:.1} GiB · caché {:.1} GiB · swap {:.1} / {:.1} GiB",
            metricas.ram().cache() / 1024.0,
            metricas.ram().swap_used() / 1024.0,
            metricas.ram().swap_total() / 1024.0,
        )
        .into(),
    );

    let disco = metricas.disk();
    app.set_disco_porcentaje(disco.usage_p() as f32);
    app.set_disco_valor(format!("{:.1} %", disco.usage_p()).into());
    app.set_disco_detalle(
        format!(
            "{}: usado {:.1} GiB · libre {:.1} GiB",
            disco.device(),
            gib(disco.used()),
            gib(disco.free())
        )
        .into(),
    );

    let redes = metricas
        .red()
        .iter()
        .map(|(nombre, red)| RedVista {
            nombre: nombre.as_str().into(),
            recibido: format!("{:.2} MB", red.received()).into(),
            enviado: format!("{:.2} MB", red.transmitted()).into(),
        })
        .collect::<Vec<_>>();
    app.set_redes(ModelRc::new(VecModel::from(redes)));
}

/// Aplica un cambio en el hilo de la interfaz. Devuelve `false` si la ventana ya se cerró.
fn en_ui(debil: &Weak<AppWindow>, cambio: impl FnOnce(&AppWindow) + Send + 'static) -> bool {
    let debil = debil.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(app) = debil.upgrade() {
            cambio(&app);
        }
    })
    .is_ok()
}

fn porcentaje(parte: f64, total: f64) -> f32 {
    if total <= 0.0 {
        0.0
    } else {
        ((parte / total) * 100.0).clamp(0.0, 100.0) as f32
    }
}

fn gib(bytes: f64) -> f64 {
    bytes / (1024.0 * 1024.0 * 1024.0)
}

#[cfg(test)]
mod tests {
    use super::porcentaje;

    #[test]
    fn el_porcentaje_se_queda_entre_cero_y_cien() {
        assert_eq!(porcentaje(5.0, 10.0), 50.0);
        assert_eq!(porcentaje(1.0, 0.0), 0.0);
        assert_eq!(porcentaje(20.0, 10.0), 100.0);
    }
}
