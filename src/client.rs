use std::sync::Arc;

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::PayloadTypeEnum;
///TODO надо подумать как рестартить клиента, эти значения придется реинициализировать
//static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
//static MESSAGES: OnceCell<UnboundedSender<Message>> = OnceCell::new();

pub struct Client
{
    runtime: Runtime,
    sender: UnboundedSender<Message>,
    pub closure: Box<dyn Fn(ClientSideMessage) + Send + Sync + 'static>

}

impl Client
{
    ///сделать трейт с мессаджами и возвращать его
    pub fn start_new<F>(addr: &str, func: F) where F: Fn(ClientSideMessage) + Send + 'static + Sync
    {
        let addr = addr.to_owned();
        let (sender, receiver) = unbounded::<Message>();
        let slf = Self
        {
            runtime: Runtime::new().unwrap(),
            sender,
            closure: Box::new(func)
        };
        slf.runtime.spawn(async move
        {
            let (ws_stream, resp) = connect_async(&addr).await.expect("Ошибка соединения с сервером");
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
                    if message.is_pong() || message.is_ping()
                    {
                        debug!("получено сообщение ping {} pong {}", message.is_ping(), message.is_pong())
                    }
                    else
                    {
                        let data = message.into_data();
                        let obj = serde_json::from_slice::<ClientSideMessage>(&data);
                        if let Ok(m) = obj
                        {
                            debug!("Клиентом получено сообщение: success: {}, payload_type: {}, payload: {:?}", m.success, m.payload_type, m.payload);
                            let cl =  &slf.closure;
                            cl(m);
                        }
                        else 
                        {
                            logger::error!("Ошибка десериализации объекта: {}", obj.err().unwrap());
                        }
                    }
                    future::ok(())
                })
            };
            pin_mut!(outgoing, incoming);
            future::select(outgoing, incoming).await;
        });
        
    }
    ///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
    /// # Examples
    /// ```
    ///start_client("ws://127.0.0.1:3010/", |message|
    ///{
    ///    logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
    ///});
    /// ```
    // pub fn start_async<F>(self, addr: &str, receiver: UnboundedReceiver<Message>)
    // {
       
    // }
    async fn start(cli: &Self, addr: String, receiver: UnboundedReceiver<Message>)
    {
        let (ws_stream, resp) = connect_async(&addr).await.expect("Ошибка соединения с сервером");
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
                if message.is_pong() || message.is_ping()
                {
                    debug!("получено сообщение ping {} pong {}", message.is_ping(), message.is_pong())
                }
                else
                {
                    let data = message.into_data();
                    let obj = serde_json::from_slice::<ClientSideMessage>(&data);
                    if let Ok(m) = obj
                    {
                        debug!("Клиентом получено сообщение: success: {}, payload_type: {}, payload: {:?}", m.success, m.payload_type, m.payload);
                        let cl =  &cli.closure;
                        cl(m);
                    }
                    else 
                    {
                        logger::error!("Ошибка десериализации объекта: {}", obj.err().unwrap());
                    }
                }
                future::ok(())
            })
        };
        pin_mut!(outgoing, incoming);
        future::select(outgoing, incoming).await;
    }

    pub async fn send_message<M: Serialize>(&self, msg: M)
    {
        let msg = serde_json::to_string(&msg).unwrap();
        let msg = Message::binary(msg);
        if let Ok(s) = self.sender.unbounded_send(msg)
        {
            
        }
        else
        {
            error!("Ошибка отправки сообщения серверу")
        }
    }
    pub async fn ping(&self)
    {
        let msg = Message::Ping([12].to_vec());
        if let Ok(s) = self.sender.unbounded_send(msg)
        {
            
        }
        else
        {
            error!("Ошибка отправки сообщения серверу")
        }
    }

}
///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
/// # Examples
/// ```
///start_client("ws://127.0.0.1:3010/", |message|
///{
///    logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
///});
/// ```
// pub fn start_client<F>(addr: &str, func: F) where F: Fn(ClientSideMessage) + Send + 'static + Sync
// {
//     let addr = addr.to_owned();
//     ASYNC_RUNTIME.spawn(async move
//     {
//         start(addr, func).await;
//     });
// }
///ws://127.0.0.1:3010/
// async fn start<F>(addr: String, func: F) where F: Fn(ClientSideMessage) + Send + 'static
// {
//     //let url = url::Url::parse(&connect_addr).unwrap();
//     let (sender, receiver) = unbounded::<Message>();
//     //tokio::spawn(read_stdin(stdin_tx));
//     let _ = MESSAGES.set(sender);
//     let (ws_stream, resp) = connect_async(&addr).await.expect("Ошибка соединения с сервером");
//     println!("Рукопожатие с сервером успешно");
//     for h in resp.headers()
//     {
//         debug!("* {}: {}", h.0.as_str(), h.1.to_str().unwrap());
//     }
//     let (write, read) = ws_stream.split();

//     let outgoing = receiver.map(Ok).forward(write);
//     let incoming = 
//     {
//         read.try_for_each(|message|
//         {
//             if message.is_pong() || message.is_ping()
//             {
//                 debug!("получено сообщение ping {} pong {}", message.is_ping(), message.is_pong())
//             }
//             else
//             {
//                 let data = message.into_data();
//                 let obj = serde_json::from_slice::<ClientSideMessage>(&data);
//                 if let Ok(m) = obj
//                 {
//                     debug!("Клиентом получено сообщение: success: {}, payload_type: {}, payload: {:?}", m.success, m.payload_type, m.payload);
//                     func(m);
//                 }
//                 else 
//                 {
//                     logger::error!("Ошибка десериализации объекта: {}", obj.err().unwrap());
//                 }
//             }
//             future::ok(())
//         })
//     };
//     pin_mut!(outgoing, incoming);
//     future::select(outgoing, incoming).await;
// }




///```
/// let _ = ClientSideMessage::from_str("тестовая строка от клиента").send().await;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSideMessage
{
    pub success: bool,
    pub payload_type: String,
    #[serde(skip_serializing_if="Option::is_none")]
    #[serde(default="default_option")]
    pub payload: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    #[serde(default="default_option")]
    pub object_name: Option<String>
}
fn default_option() -> Option<String>
{
    None
}

impl ClientSideMessage
{
    pub fn from_str(msg: &str) -> Self
    {
        Self
        {
            success: true,
            payload_type: PayloadTypeEnum::String.to_string(),
            payload: Some(msg.to_owned()),
            object_name: None,
        }
    }
    pub fn get_payload_type(&self) -> PayloadTypeEnum
    {
        let pl = &self.payload_type;
        pl.into()
    }
    pub fn from_number(msg: i64) -> Self
    {
        Self
        {
            success: true,
            payload_type: PayloadTypeEnum::Number.to_string(),
            payload: Some(msg.to_string()),
            object_name: None

        }
    }
    pub fn command(msg: &str) -> Self
    {
        Self
        {
            success: true,
            payload_type: PayloadTypeEnum::Command.to_string(),
            payload: Some(msg.to_string()),
            object_name: None

        }
    }
}


#[cfg(test)]
mod test
{
    use std::time::Duration;

    use crate::{client::Client, ClientSideMessage};

    #[tokio::test]
    async fn test_client()
    {
        logger::StructLogger::initialize_logger();
        let client = Client::start_new("ws://127.0.0.1:3010/", |message|
        {
            logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
        });
        // super::start_client("ws://127.0.0.1:3010/", |message|
        // {
        //     logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
        // });
        loop 
        {
            std::thread::sleep(Duration::from_secs(5));
            let _ = client.ping().await;
        }
    }
}