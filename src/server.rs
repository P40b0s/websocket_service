use logger::{backtrace, debug, error, info};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{runtime::Runtime, sync::Mutex};
use tokio_tungstenite::tungstenite::{handshake::server::{Request, Response}, Message};
use std::{borrow::Cow, sync::{Arc}, collections::HashMap, fmt::Display, error::Error};
use std::ops::ControlFlow;
use std::{net::SocketAddr, path::PathBuf};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::{pin_mut, SinkExt};
use futures::{stream::StreamExt, future, TryStreamExt};

static WS_STATE: Lazy<Arc<Mutex<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(Mutex::new(HashMap::new()))
});
static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
static MESSAGE_RECEIVER: Lazy<Mutex<HashMap<SocketAddr, UnboundedReceiver<ServerSideMessage>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub trait PayloadType
{
    fn get_type() -> String;
}

///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
pub struct Server;
impl Server
{
    pub fn start_server(host: &str)
    {
        let addr = host.to_string(); 
        ASYNC_RUNTIME.spawn(async move
        {
            debug!("Стартую websocket...");
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = tokio::net::TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Ошибка привязки");
            debug!("Websocet доступен на : {}", &addr);
            while let Ok((stream, _)) = listener.accept().await 
            {
                debug!("НОВОЕ СОЕДИНЕНИЕ!");
                Self::accept_connection(stream).await;
            }
        });
    }
    async fn add_message_receiver(socket: &SocketAddr, receiver: UnboundedReceiver<ServerSideMessage>)
    {
        let mut mr_guard = MESSAGE_RECEIVER.lock().await;
        let _ = mr_guard.insert(socket.clone(), receiver);
    }
    async fn add_message_sender(socket: &SocketAddr, sender: UnboundedSender<Message>)
    {
        let mut guard =   WS_STATE.lock().await;
        guard.insert(socket.clone(), sender);
        drop(guard);

    }

    async fn accept_connection(stream: tokio::net::TcpStream)
    {
        let (s, r) = unbounded::<ServerSideMessage>();
        let addr = stream.peer_addr().expect("Соединение должно иметь исходящий ip адрес");
        Self::add_message_receiver(&addr, r).await;
        let headers_callback = |req: &Request, mut response: Response| 
        {
            debug!("Получен новый ws handshake от {}", &addr);
            debug!("Путь запроса: {}", req.uri().path());
            debug!("Хэдеры запроса:");
            for (ref header, _value) in req.headers() 
            {
                debug!("* {}: {:?}", header, _value);
            }
            //let headers = response.headers_mut();
            //headers.append("MyCustomHeader", ":)".parse().unwrap());
            Ok(response)
        };
        let ws_stream = tokio_tungstenite::accept_hdr_async(stream, headers_callback)
            .await
            .expect("Ошибка handsnake при извлечении данных из websocket");
        let (sender, receiver) = unbounded();
        Self::add_message_sender(&addr, sender).await;
        let (outgoing, incoming) = ws_stream.split();
        let broadcast_incoming = incoming.try_for_each(|msg| 
        {
            if !msg.is_ping() && !msg.is_pong() && !msg.is_empty() && !msg.is_close()
            {
                if let Ok(m) = msg.clone().into_text()
                {
                    debug!("Сервером получено сообщение: {}", m);
                }
                let data = msg.into_data();
                let deserialize = serde_json::from_slice::<super::ServerSideMessage>(&data);
                if let Ok(d) = deserialize
                {
                    let _ = s.unbounded_send(d);
                    logger::debug!("Полученный объект отправлен сервером в поток сообщений {}",  s.len());
                }
                else
                {
                    logger::error!("Ошибка десериализации обьекта {:?} поступившего от клиента {} ", &data, &addr);
                }
            }
            future::ok(())
        });
        let receive_from_others = receiver.map(Ok).forward(outgoing);
        pin_mut!(broadcast_incoming, receive_from_others);
        let _ = future::select(broadcast_incoming, receive_from_others).await;
        WS_STATE.lock().await.remove(&addr);
        debug!("Клиент {} отсоединен", &addr);
    }

}



#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSideMessage
{
    ///если success false то в payload будет лежать строка с ошибкой иначе там будет объект на сонове payload_type
    pub success: bool,
    ///error | payload | somestruct
    //#[serde(deserialize_with="payload_type_deserializer")]
    //#[serde(serialize_with="payload_type_serializer")]
    pub payload_type: String,
    pub payload: Option<String>
}

impl ServerSideMessage
{
    pub fn error(error: &str) -> Self
    {
        Self
        {
            success: false,
            payload_type: "error".to_owned(),
            payload: Some(error.to_owned())

        }
    }
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
    pub fn from_struct<T: Serialize + Clone + PayloadType>(val: &T) -> Self
    {
        let ser = serde_json::to_value(val);
        let mut tp = "".to_owned();
        if ser.is_err()
        {
            let error = ser.err().unwrap();
            Self::error(&error.to_string())
        }
        else
        {
            let obj = ser.unwrap();
            if obj.is_object()
            {
                tp = T::get_type();
            }
            else if obj.is_array()
            {
                tp = [T::get_type(), " array".to_owned()].concat();
            }
            else if obj.is_string()
            {
                tp = "string".to_owned();
            }
            else if obj.is_number()
            {
                tp = "number".to_owned();
            }
            else 
            {
                return Self::error(&format!("Тип объекта {:?} пока не обрабатывается", obj.as_str()));
            }
            Self
            {
                success: true,
                payload_type: tp,
                payload: serde_json::to_string_pretty(val).ok()
            }
            
        }
    }

    pub fn on_receive_msg<F>(f: F ) where F: Fn(SocketAddr, ServerSideMessage) + Send + 'static
    {
        ASYNC_RUNTIME.spawn(async move
        {
            loop 
            {
                let mut guard = MESSAGE_RECEIVER.lock().await;
                for (addr,  recv) in guard.iter_mut()
                {
                    if let Ok(m) = recv.try_next()
                    {
                        if let Some(m) = m
                        {
                            f(addr.clone(), m);
                        }
                    }
                }
            }
        });
    }

    async fn broadcast_all(msg: &Message)
    {
        // We want to broadcast the message to everyone except ourselves.
        let state = WS_STATE
        .lock()
        .await;
        let receivers = state
        .iter()
        .map(|(_, ws_sink)| ws_sink);
        for recp in receivers
        {
            recp.unbounded_send(msg.clone()).unwrap();
        }
    }
    async fn message_to_all_except_sender(addr: &SocketAddr, msg: &Message)
    {
        // We want to broadcast the message to everyone except ourselves.
        let state = WS_STATE
            .lock()
            .await;
            let receivers = state
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &addr)
            .map(|(_, ws_sink)| ws_sink);

        for recp in receivers
        {
            let ms = msg.to_string();
            recp.unbounded_send(msg.clone()).unwrap();
        }
    }

    pub async fn send(&self, addr: &SocketAddr)
    {
        let msg = json!(self);
        let msg = Message::binary(msg.to_string());
        if let Some(sender) = WS_STATE.lock().await.get(addr) 
        {
            sender.unbounded_send(msg).unwrap();
        }
        else
        {
            error!("Ошибка отправки сообщения клиенту {}, возможно он уже отключен.", addr.ip().to_string())
        }
    }
    pub async fn send_to_all(&self)
    {
        let msg = json!(self);
        let mm = Message::binary(msg.to_string());
        Self::broadcast_all(&mm).await;
    }
}










#[cfg(test)]
mod tests
{
    use logger::{debug, error};
    use serde::{Deserialize, Serialize};

    use super::{ServerSideMessage, PayloadType};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TestStruct
    {
        pub success: bool,
        pub test_type: String,
        pub payload: Option<String>
    }
    impl PayloadType for TestStruct
    {
        fn get_type() -> String 
        {
            "TestStruct".to_owned()
        }
    }


    #[test]
    fn test_deserialize_message()
    {
        logger::StructLogger::initialize_logger();
        let test = TestStruct
        {
            success: true,
            test_type: "Новый тестовый тип!".to_owned(),
            payload: Some("iwjeoiwjeoifjweof".to_owned())
        };
        let n = ServerSideMessage::from_struct(&test);
        debug!("type: {}, payload: {:?}", n.payload_type, n.payload);
        
        
    }

    #[test]
    fn test_deserialize_message_by_url()
    {
        logger::StructLogger::initialize_logger();
        
    }
}

