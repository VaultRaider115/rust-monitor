use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

pub type Sesion = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Abre el WebSocket y consume el saludo que manda el servidor al conectar.
pub async fn abrir(url: &str) -> Result<(Sesion, String), String> {
    let (mut sesion, _respuesta) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|error| format!("No se pudo conectar a {url}: {error}"))?;

    let saludo = match sesion.next().await {
        Some(Ok(Message::Text(texto))) => texto.to_string(),
        Some(Ok(otro)) => format!("{otro:?}"),
        Some(Err(error)) => return Err(error.to_string()),
        None => return Err("El servidor cerró la conexión al saludar".to_string()),
    };

    Ok((sesion, saludo))
}

/// Envía `metrics` (el único mensaje que entiende el servidor) y devuelve el
/// texto de respuesta: el JSON de métricas o un mensaje de error del servidor.
pub async fn pedir_metricas(sesion: &mut Sesion) -> Result<String, String> {
    sesion
        .send(Message::Text("metrics".into()))
        .await
        .map_err(|error| error.to_string())?;

    loop {
        match sesion.next().await {
            Some(Ok(Message::Text(texto))) => return Ok(texto.to_string()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) | None => {
                return Err("El servidor cerró la conexión".to_string());
            }
            Some(Ok(otro)) => return Err(format!("Mensaje inesperado: {otro:?}")),
            Some(Err(error)) => return Err(error.to_string()),
        }
    }
}
