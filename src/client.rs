use std::sync::atomic::AtomicBool;
use futures::SinkExt;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::message::WebsocketMessage;

static SENDER: OnceCell<UnboundedSender<Message>> = OnceCell::new();
static RECEIVER: OnceCell<Mutex<UnboundedReceiver<WebsocketMessage>>> = OnceCell::new();
static RECEIVER_FN_ISACTIVE: AtomicBool = AtomicBool::new(false);

pub struct Client{}
impl Client
{
    ///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
    /// # Examples
    /// ```
    ///start_client("ws://127.0.0.1:3010/", |message|
    ///{
    ///    logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
    ///});
    /// ```
    pub async fn start_client(addr: &str)
    {
        let addr = addr.to_owned();
        tokio::spawn(async move
        {
            Self::start(addr).await;
        });
    }
    ///ws://127.0.0.1:3010/
    async fn start(addr: String)
    {
        let (sender, local_receiver) = unbounded::<Message>();
        let (mut local_sender, receiver) = unbounded::<WebsocketMessage>();
        let _ = SENDER.set(sender);
        let _ = RECEIVER.set(Mutex::new(receiver));
        let (ws_stream, resp) = connect_async(&addr).await
        .expect("Ошибка соединения с сервером");
        println!("Рукопожатие с сервером успешно");
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
                if message.is_pong()
                {
                    debug!("получено сообщение pong {}",message.is_pong())
                }
                else
                {
                    let msg =  TryInto::<WebsocketMessage>::try_into(&message);
                    if let Ok(m) = msg
                    {
                    
                        debug!("Клиентом получено сообщение: success: {}, command: {}, method: {}", m.success, m.command.target, m.command.method);
                        if RECEIVER_FN_ISACTIVE.load(std::sync::atomic::Ordering::SeqCst)
                        {
                            let _ = local_sender.send(m);
                        }
                    }
                    else 
                    {
                        logger::error!("Ошибка десериализации объекта: {}", msg.err().unwrap());
                    }
                }
                future::ok(())
            })
        };
        pin_mut!(outgoing, incoming);
        future::select(outgoing, incoming).await;
    }

    pub async fn on_receive_message<F: Fn(WebsocketMessage) + Send + Sync + 'static>(f: F )
    {
        RECEIVER_FN_ISACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
        tokio::spawn(async move
        {
            let receiver = RECEIVER.get().unwrap();
            let mut r = receiver.lock().await;
            while let Some(msg) = r.next().await 
            {
                f(msg);
            }
        });
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
    use super::Client;

    #[tokio::test]
    async fn test_client()
    {
        logger::StructLogger::initialize_logger();
        super::Client::start_client("ws://127.0.0.1:3010/").await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let _ = Client::ping().await;
        }
    }
}