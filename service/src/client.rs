use std::collections::HashMap;
use anyhow::Context;
use futures::Future;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt};
use logger::{backtrace,  error};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::retry;

static SENDER: OnceCell<Mutex<HashMap<String, UnboundedSender<Message>>>> = OnceCell::new();
static IS_CONNECTED: OnceCell<Mutex<HashMap<String, bool>>> = OnceCell::new();


pub trait Client<T> where T: serde::Serialize + Send + Sync, for <'de> T : serde::Deserialize<'de> + Sized + Send
{
    fn get_id() -> &'static str;
    fn start_client<F>(addr: &str, f:F)  -> impl Future<Output = ()> + Send
    where F:  Send + Sync + Clone + 'static + Fn(T)
    {
        let addr = addr.to_owned();
        let cli_id = Self::get_id();
        async move
        {
            tokio::spawn(async move
            {
                loop
                {
                    start(cli_id, addr.clone(), f.clone(),0, 15).await;
                }
            });
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    fn is_connected() -> impl Future<Output = bool> + Send
    {
        async 
        {
            let guard = IS_CONNECTED.get_or_init(|| Mutex::new(HashMap::new())).lock().await;
            if let Some(curr) = guard.get(Self::get_id())
            {
                return *curr
            }
            else 
            {
                false
            }
        }
    }
    fn send_message(wsmsg: T) -> impl Future<Output = ()> + Send
    {
        async move 
        {
            if Self::is_connected().await
            {
                let mut message: Vec<u8> = Vec::new();
                let _ = serde_json::to_writer(&mut message, &wsmsg);
                let message = Message::Binary(message);
                let sender = SENDER.get().unwrap().lock().await;
                let curr_sender = sender.get(Self::get_id()).unwrap();
                let sended = curr_sender.unbounded_send(message);
                if sended.is_ok()
                {
                    return ();
                }
                else
                {
                    logger::error!("Ошибка отправки сообщения {}", sended.err().unwrap());
                    return ();
                }
            }
            else 
            {
                logger::error!("Ошибка отправки сообщения, нет подключения к серверу");
                return ();
            }
        }
    }
    fn ping() -> impl Future<Output = ()> + Send
    {
        async 
        {
            let guard = IS_CONNECTED.get_or_init(|| Mutex::new(HashMap::new())).lock().await;
            if let Some(curr) = guard.get(Self::get_id())
            {
                if *curr == true
                {
                    let msg = Message::Ping([12].to_vec());
                    let sender = SENDER.get().unwrap().lock().await;
                    let curr_sender = sender.get(Self::get_id()).unwrap();
                    let _ = curr_sender.unbounded_send(msg);
                }
                else
                {
                    error!("Ошибка отправки сообщения, нет подключения к серверу");
                }
            }
            else 
            {
                error!("Ошибка отправки сообщения, нет подключения к серверу");
            }
        }
    }
}

async fn start<F, T>(cli_id: &str, addr: String, f:F, attempts: u8, delay: u64) -> bool 
where T: serde::Serialize + Send, for <'de> T : serde::Deserialize<'de> + Sized + Send, F:  Send + Clone + 'static + Fn(T)
{
    let (sender, local_receiver) = unbounded::<Message>();
    if let Some(s) = SENDER.get()
    {
        let mut guard = s.lock().await;
        guard.insert(cli_id.to_owned(), sender.clone());
    }
    else
    {
        let mut hm: HashMap<String, UnboundedSender<Message>> = HashMap::new();
        hm.insert(cli_id.to_owned(), sender.clone());
        let _ = SENDER.set(Mutex::new(hm));
    }
    let connected = retry(attempts, delay, || connect_async(&addr)).await;
    if let Err(e) = connected.as_ref()
    {
        error!("Ошибка подключения к серверу websocket по адресу {} -> {}", &addr, e.to_string());
        return false;
    }
    let mut conn = IS_CONNECTED.get_or_init(|| Mutex::new(HashMap::new())).lock().await;
    conn.insert(cli_id.to_owned(), true);
    drop(conn);
    let (ws_stream, resp) = connected.unwrap();
    logger::debug!("Рукопожатие с сервером успешно");
    for h in resp.headers()
    {
        logger::debug!("* {}: {}", h.0.as_str(), h.1.to_str().unwrap());
    }
    let (write, read) = ws_stream.split();
    //сообщения полученные по каналу local_receiver'ом форвардятся прямо в вебсокет
    let send_to_ws = local_receiver.map(Ok).forward(write);
    let fun = f.clone();
    //для каждого входяшего сообщения по вебсокет производим обработку
    let from_ws = 
    {
        read.for_each(|message| async
        {
            if let Ok(message) = message
            {
                if message.is_binary()
                {
                    let msg = serde_json::from_slice::<T>(&message.into_data()).with_context(|| format!("Данный объект отличается от того который вы хотите получить"));
                    if let Ok(m) = msg
                    {
                        let fun = fun.clone();
                        fun(m)
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
    let mut conn = IS_CONNECTED.get().unwrap().lock().await;
    conn.remove(cli_id);
    let mut snd = SENDER.get().unwrap().lock().await;
    snd.remove(cli_id);
    logger::warn!("Сервер недоступен! повторная попытка подключения");
    return false;
}

#[cfg(test)]
mod test
{
    
}