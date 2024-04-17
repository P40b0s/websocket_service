use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};
use futures::{future::BoxFuture, Future, FutureExt, SinkExt};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{backtrace, debug, error, warn};
use once_cell::{sync::{Lazy, OnceCell}};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::{message::Converter, retry};
//TODO Пока могу обслужить только клиента в единственном экземпляре
static SENDER: OnceCell<Mutex<UnboundedSender<Message>>> = OnceCell::new();
static IS_CONNECTED: AtomicBool = AtomicBool::new(false);
//static SENDER2: Lazy<Mutex<HashMap<String, UnboundedSender<Message>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
//static IS_CONNECTEDD : Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub trait Client<T: Converter>
{
    fn start_client<F>(addr: &str, f:F)  -> impl Future<Output = ()> + Send
    where F:  Send + Sync + Copy + 'static + Fn(T)
    {
        let addr = addr.to_owned();
        async move 
        {
            // if IS_CONNECTEDD.lock().await.get(&addr).is_some()
            // {
            //     logger::error!("Ошибка! уже активен один клиент подключенный к {}", &addr);
            //     return ();
            // }
            // else 
            // {
            //     let mut guard = IS_CONNECTEDD.lock().await;
            //     guard.insert(addr.clone(), false);
            // }
            tokio::spawn(async move
            {
                loop
                {
                    start(addr.clone(), f,0, 15).await;
                }
            });
        }
    }
    fn is_connected() -> bool
    {
        IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
    }
    ///ws://127.0.0.1:3010/
    
    fn send_message(wsmsg: T) -> impl Future<Output = ()> + Send
    {
        async {
        if !IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            error!("Ошибка отправки сообщения, нет подключения к серверу")
        }
        else 
        {
            let message = wsmsg.to_binary();
            let message = Message::Binary(message);
            let _ = SENDER.get().unwrap().lock().await.unbounded_send(message);
        }
    }
    }
    fn ping() -> impl Future<Output = ()> + Send
    {
        async {
        if IS_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            let msg = Message::Ping([12].to_vec());
            let _ = SENDER.get().unwrap().lock().await.unbounded_send(msg);
        }
    }
    }
}

async fn start<F, T: Converter>(addr: String, f:F, attempts: u8, delay: u64) -> bool
where F:  Send + Copy + 'static + Fn(T)
{
    let (sender, local_receiver) = unbounded::<Message>();
    // let current_id = Client::<T>::client_id();
    // if let Some(s) = SENDER2.get()
    // {
    //     let mut guard = s.lock().await;
    //     if let Some(value) = guard.get_mut(&current_id)
    //     {
    //         *value = sender.clone()
    //     }
    // }
    // else
    // {
    //     let _ = SENDER.set(Mutex::new(sender.clone()));
    // }
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
                    let msg = T::from_binary(&message.into_data());
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

#[cfg(test)]
mod test
{
    use logger::debug;
    //use crate::WebsocketMessage;
    use super::Client;

    enum Packet
    {
        Test1
    }
    // #[tokio::test]
    // async fn test_client()
    // {
    //     logger::StructLogger::initialize_logger();
    //     super::Client::start_client("ws://127.0.0.1:3010/", receiver).await;
    //     loop 
    //     {
    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //         let _ = Client::ping().await;
    //     }
    // }
    // fn receiver(ms: WebsocketMessage)
    // {
    //     debug!("клиенту поступило новое сообщение {:?}", ms);
    // }
}