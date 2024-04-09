use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};

use futures::{future, pin_mut, SinkExt, StreamExt};
use logger::{debug, error, info};
use once_cell::sync::Lazy;
use tokio::{spawn, sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, RwLock}};
use tokio_tungstenite::connect_async;
use tungstenite::{connect, Message};
use url::Url;
use crate::WebsocketMessage;

static CLIENTS: Lazy<Arc<RwLock<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(RwLock::new(HashMap::new()))
});

async fn add_message_sender(socket: &SocketAddr, sender: UnboundedSender<Message>)
{
    let mut guard =   CLIENTS.write().await;
    guard.insert(socket.clone(), sender);
    drop(guard);
}
pub async fn start_client() -> Result<(), Box<dyn std::error::Error>>
{
    let url = "ws://127.0.0.1:3010";
    let (mut ws_stream, _) = connect_async(url).await?;
    let text = "Hello, World!";
    println!("Sending: \"{}\"", text);
    ws_stream.send(Message::text(text)).await?;
    let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
    println!("Received: {:?}", msg);
    Ok(())
}

#[cfg(test)]
mod test
{
    use logger::debug;
    use crate::WebsocketMessage;
    #[tokio::test]
    async fn test_client()
    {
        logger::StructLogger::initialize_logger();
        super::start_client().await;
        loop 
        {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}