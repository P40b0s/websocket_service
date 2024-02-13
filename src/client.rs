use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use logger::{debug, error};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{ runtime::Runtime};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
static MESSAGES: OnceCell<UnboundedSender<Message>> = OnceCell::new();
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
                debug!("success: {}, payload_type: {}, payload: {:?}", m.success, m.payload_type, m.payload);
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





#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSideMessage
{
    pub success: bool,
    pub payload_type: String,
    pub payload: Option<String>
}

impl ClientSideMessage
{
    pub fn from_str(msg: &str) -> Self
    {
        Self
        {
            success: true,
            payload_type: "string".to_owned(),
            payload: Some(msg.to_owned())

        }
    }
    pub fn from_number(msg: &i32) -> Self
    {
        Self
        {
            success: true,
            payload_type: "number".to_owned(),
            payload: Some(msg.to_string())

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
    use tokio::runtime::Runtime;


    #[tokio::test]
    pub async fn test_connection()
    {
         logger::StructLogger::initialize_logger();
       
           
                //super::start_client().await;
           
           
        loop {
        }
        println!("ВСЕ!");
    }
}