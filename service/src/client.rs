use std::sync::{atomic::AtomicBool, Arc};
use futures::{future::BoxFuture, FutureExt, SinkExt};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{backtrace, debug, error, warn};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::{message::WebsocketMessage, retry};

static SENDER: OnceCell<Mutex<UnboundedSender<Message>>> = OnceCell::new();
static IS_CONNECTED: AtomicBool = AtomicBool::new(false);
//static RECEIVER: OnceCell<Mutex<UnboundedReceiver<WebsocketMessage>>> = OnceCell::new();
//static RECEIVER_FN_ISACTIVE: AtomicBool = AtomicBool::new(false);

pub struct Client;
impl Client
{
    /// # Examples
    /// ```
    /// async run_client()
    /// {
    ///     Client::start_client("ws://127.0.0.1:3010/", on_client_receive).await;
    ///     fn on_client_receive(msg: WebsocketMessage)
    ///     {
    ///         debug!("Клиентом1 полчено сообщение через канал {} {:?}", &msg.command.target, &msg.command.payload);
    ///         ()
    ///     }
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
            loop
            {
                Self::start(addr.clone(), f,0, 15).await;
            }
        });
    }
    pub fn is_connected() -> bool
    {
        IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
    }
     /// # Examples
    /// ```
    /// async run_client()
    /// {
    ///     Client::start_client_with_retry("ws://127.0.0.1:3010/", on_client_receive, 20, 15).await;
    ///     fn on_client_receive(msg: WebsocketMessage)
    ///     {
    ///         debug!("Клиентом1 полчено сообщение через канал {} {:?}", &msg.command.target, &msg.command.payload);
    ///         ()
    ///     }
    ///     loop
    ///     {
    ///         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    ///         let cli_wsmsg: WebsocketMessage = "test_client_cmd:test_client_method".into();
    ///         _ = Client::send_message(&cli_wsmsg).await;
    ///     }
    /// }
    /// ```
    pub async fn start_client_with_retry<F>(addr: &str, f:F, attempts: u8, delay: u64)
    where F:  Send + Sync + Copy + 'static + Fn(WebsocketMessage)
    {
        let addr = addr.to_owned();
        tokio::spawn(async move
        {
            while Self::start(addr.clone(), f, attempts, delay).await == false
            {
                Self::start(addr.clone(), f, attempts, delay).await;
            }
        });
    }
    ///ws://127.0.0.1:3010/
    async fn start<F>(addr: String, f:F, attempts: u8, delay: u64) -> bool
    where F:  Send + Copy + 'static + Fn(WebsocketMessage)
    {
        let (sender, local_receiver) = unbounded::<Message>();
        if let Some(s) = SENDER.get()
        {
            let mut guard = s.lock().await;
            *guard = sender.clone();
        }
        else
        {
            let _ = SENDER.set(Mutex::new(sender.clone()));
        }
        //let snd = SENDER.get_or_init(|| Mutex::new(sender.clone())).lock().await;
        //let _ = SENDER.set(sender.clone());
        let connected = retry(attempts, delay, || connect_async(&addr)).await;
        if let Err(e) = connected.as_ref()
        {
            error!("Ошибка подключения к серверу websocket по адресу {} -> {}", &addr, e.to_string());
            return false;
        }
        IS_CONNECTED.store(true, std::sync::atomic::Ordering::SeqCst);
        let (ws_stream, resp) = connected.unwrap();
        debug!("Рукопожатие с сервером успешно");
        for h in resp.headers()
        {
            debug!("* {}: {}", h.0.as_str(), h.1.to_str().unwrap());
        }
        let (write, read) = ws_stream.split();
        //сообщения полученные по каналу local_receiver'ом форвардятся прямо в вебсокет
        let send_to_ws = local_receiver.map(Ok).forward(write);
        //для каждого входяшего сообщения по вебсокет производим обработку
        let from_ws = 
        {
            read.for_each(|message| async 
            {
                if let Ok(message) = message
                {
                    if message.is_binary()
                    {
                        let msg = WebsocketMessage::from_transport_message(&message);
                        if let Ok(m) = msg
                        {
                            f(m)
                        }
                        else 
                        {
                            logger::error!("Ошибка десериализации объекта на клиенте: {}", msg.err().unwrap());
                        }
                    }
                    //logger::info!("получено сообщение от сервера {:?}", message);
                }
                else
                {
                    logger::error!("Ошибка чтения сообщения! {} -> {}", message.err().unwrap().to_string(), backtrace!());
                }
            })
        };
        pin_mut!(send_to_ws, from_ws);
        future::select(send_to_ws, from_ws).await;
        IS_CONNECTED.store(false, std::sync::atomic::Ordering::SeqCst);
        warn!("Сервер недоступен! повторная попытка подключения");
        return false;
    }

    pub async fn send_message(wsmsg: &WebsocketMessage)
    {
        if !IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            error!("Ошибка отправки сообщения, нет подключения к серверу")
        }
        else 
        {
            if let Ok(msg) = wsmsg.to_transport_message()
            {
                let _ = SENDER.get().unwrap().lock().await.unbounded_send(msg);
            }
            else
            {
                error!("Ошибка отправки сообщения серверу")
            }
        }
    }
    pub async fn ping()
    {
        if IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            let msg = Message::Ping([12].to_vec());
            let _ = SENDER.get().unwrap().lock().await.unbounded_send(msg);
        }
    }
}


#[cfg(test)]
mod test
{
    use logger::debug;
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
        debug!("клиенту поступило новое сообщение {:?}", ms);
    }
}