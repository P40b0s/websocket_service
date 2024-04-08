use std::sync::{atomic::AtomicBool, Arc};
use futures::SinkExt;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::message::WebsocketMessage;

static SENDER: OnceCell<UnboundedSender<Message>> = OnceCell::new();
//static RECEIVER: OnceCell<Mutex<UnboundedReceiver<WebsocketMessage>>> = OnceCell::new();
//static RECEIVER_FN_ISACTIVE: AtomicBool = AtomicBool::new(false);

pub struct Client;
impl Client
{
    /// # Examples
    /// ```
    /// async run_client()
    /// {
    ///     Client::start_client("ws://127.0.0.1:3010/").await;
    ///     let _ = Client::on_receive_message(|msg|
    ///     {
    ///         println!("Клиентом полчено сообщение через канал {}", &msg.command.target);
    ///     }).await;
    ///     loop
    ///     {
    ///         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    ///         let cli_wsmsg: WebsocketMessage = "test_client_cmd:test_client_method".into();
    ///         _ = Client::send_message(&cli_wsmsg).await;
    ///     }
    /// }
    /// ```
    pub async fn start_client<F>(addr: &str, f:F)
    where F:  Send + Sync + Copy + 'static + Fn(WebsocketMessage)
    {
        let addr = addr.to_owned();
        tokio::spawn(async move
        {
            Self::start(addr, f).await;
        });
    }
    ///ws://127.0.0.1:3010/
    async fn start<F>(addr: String, f:F)
    where F:  Send + Copy + 'static + Fn(WebsocketMessage)
    {
        let (sender, local_receiver) = unbounded::<Message>();
        let _ = SENDER.set(sender);
        let (ws_stream, resp) = connect_async(&addr).await
        .expect("Ошибка соединения с сервером");
        debug!("Рукопожатие с сервером успешно");
        for h in resp.headers()
        {
            debug!("* {}: {}", h.0.as_str(), h.1.to_str().unwrap());
        }
        let (write, read) = ws_stream.split();
        let outgoing = local_receiver.map(Ok).forward(write);
        let incoming = 
        {
           
            read.try_for_each(|message|
            {
                debug!("Клиентом получено сообщение  {:?}",&message);
                if message.is_pong()
                {
                    debug!("Клиентом получено сообщение pong {}",message.is_pong())
                }
                else
                {
                    let msg =  TryInto::<WebsocketMessage>::try_into(&message);
                    if let Ok(m) = msg
                    {
                        f(m)
                    }
                    else 
                    {
                        logger::error!("Ошибка десериализации объекта на клиенте: {}", msg.err().unwrap());
                    }
                }
                future::ok(())
            })
        };
        pin_mut!(outgoing, incoming);
        future::select(outgoing, incoming).await;
    }

    pub async fn send_message(wsmsg: &WebsocketMessage)
    {
        if let Ok(msg) = wsmsg.try_into()
        {
            let _ = SENDER.get().unwrap().unbounded_send(msg);
        }
        else
        {
            error!("Ошибка отправки сообщения серверу")
        }
    }
    pub async fn ping()
    {
        let msg = Message::Ping([12].to_vec());
        let _ = SENDER.get().unwrap().unbounded_send(msg);
    }
}


#[cfg(test)]
mod test
{
    use crate::WebsocketMessage;

    use super::Client;

    #[tokio::test]
    async fn test_client()
    {
        logger::StructLogger::initialize_logger();
        super::Client::start_client("ws://127.0.0.1:3010/", receiver).await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let _ = Client::ping().await;
        }
    }

    fn receiver(ms: WebsocketMessage)
    {
        ()
    }
}