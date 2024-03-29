use std::sync::atomic::AtomicBool;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::{message::WebsocketMessage};
///TODO надо подумать как рестартить клиента, эти значения придется реинициализировать
static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
static MESSAGES: OnceCell<UnboundedSender<Message>> = OnceCell::new();
static CLIENT_STATUS : AtomicBool = AtomicBool::new(false);

pub trait MessageLayer
{
    async fn ping(&self);
    async fn send_message<M: Serialize>(&self, msg: M);
}

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
    start(addr).await;
}
///ws://127.0.0.1:3010/
async fn start(addr: String)
{
    let (sender, receiver) = unbounded::<Message>();
    let _ = MESSAGES.set(sender);
    let (ws_stream, resp) = connect_async(&addr).await
    .expect("Ошибка соединения с сервером");
    println!("Рукопожатие с сервером успешно");
    for h in resp.headers()
    {
        debug!("* {}: {}", h.0.as_str(), h.1.to_str().unwrap());
    }
    let (write, read) = ws_stream.split();
    let outgoing = receiver.map(Ok).forward(write);
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
                let data = message.into_data();
                let r = flexbuffers::Reader::get_root(data.as_slice()).unwrap();
                println!("{}", r);
                let msg = WebsocketMessage::deserialize(r);
                if let Ok(m) = msg
                {
                    debug!("Клиентом получено сообщение: success: {}, command: {}, method: {}", m.success, m.command.target, m.command.method);
                    println!("СДЕЛАТЬ МЕТОД ДЛЯ ОБРАБОТКИ!");
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

pub trait ClientMessage where Self: Sized + Serialize
{
    async fn send_message(self)
    {
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.serialize(&mut s).unwrap();
        let obj = s.view();
        let msg = Message::binary(obj);
        let _ = MESSAGES.get().unwrap().unbounded_send(msg);
        // {

        // }
        // else
        // {
        //     error!("Ошибка отправки сообщения серверу")
        // }
    }
    async fn ping()
    {
        let msg = Message::Ping([12].to_vec());
        let _ = MESSAGES.get().unwrap().unbounded_send(msg);
        // if let Ok(send) = MESSAGES.get().unwrap().unbounded_send(msg)
        // {
            
        // }
        // else
        // {
        //     error!("Ошибка отправки сообщения серверу")
        // }
    }
}

impl ClientMessage for WebsocketMessage {}


#[cfg(test)]
mod test
{
    use std::time::Duration;

    use crate::{client::ClientMessage, message::WebsocketMessage};

    #[tokio::test]
    async fn test_client()
    {
        logger::StructLogger::initialize_logger();
        tokio::spawn(async
        {
            super::start_client("ws://127.0.0.1:3010/").await;
        });
        
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let _ = WebsocketMessage::ping().await;
        }
    }
}