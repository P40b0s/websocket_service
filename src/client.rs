use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::PayloadTypeEnum;

static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
static MESSAGES: OnceCell<UnboundedSender<Message>> = OnceCell::new();
///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
/// # Examples
/// ```
///start_client("ws://127.0.0.1:3010/", |message|
///{
///    logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
///});
/// ```
pub fn start_client<F>(addr: &str, func: F) where F: Fn(ClientSideMessage) + Send + 'static + Sync
{
    let addr = addr.to_owned();
    ASYNC_RUNTIME.spawn(async move
    {
        start(addr, func).await;
    });
}
///ws://127.0.0.1:3010/
async fn start<F>(addr: String, func: F) where F: Fn(ClientSideMessage) + Send + 'static
{
    //let url = url::Url::parse(&connect_addr).unwrap();
    let (sender, receiver) = unbounded::<Message>();
    //tokio::spawn(read_stdin(stdin_tx));
    let _ = MESSAGES.set(sender);
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
            let data = message.into_data();
            let obj = serde_json::from_slice::<ClientSideMessage>(&data);
            if let Ok(m) = obj
            {
                debug!("Клиентом получено сообщение: success: {}, payload_type: {}, payload: {:?}", m.success, m.payload_type, m.payload);
                func(m);
            }
            else 
            {
                logger::error!("Ошибка десериализации объекта: {}", obj.err().unwrap());
            }
            future::ok(())
        })
    };
    pin_mut!(outgoing, incoming);
    future::select(outgoing, incoming).await;
}




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
    pub async fn send(&self)
    {
        let msg = json!(self);
        let msg = Message::binary(msg.to_string());
        if let Some(sender) = MESSAGES.get() 
        {
            sender.unbounded_send(msg).unwrap();
        }
        else
        {
            error!("Ошибка отправки сообщения серверу")
        }
    }
}


#[cfg(test)]
mod test
{

}