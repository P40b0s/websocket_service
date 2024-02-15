use logger::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{runtime::Runtime, sync::Mutex};
use tokio_tungstenite::tungstenite::{handshake::server::{Request, Response}, Message};
use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};
use std::net::SocketAddr;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::pin_mut;
use futures::{stream::StreamExt, future, TryStreamExt};

use crate::PayloadTypeEnum;
///Список подключенных клиентов с каналом для оправки им сообщений
static WS_STATE: Lazy<Arc<Mutex<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(Mutex::new(HashMap::new()))
});
///рантай мдля запуска асинхронных операций
static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
///Канал получаемых от клиентов сообщений 
static MESSAGE_RECEIVER: Lazy<Mutex<HashMap<SocketAddr, UnboundedReceiver<ServerSideMessage>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
///Флаг что сообщения из очереди обрабатываются,  
///ставиться автоматически при вызове замыкания обработки сообщений
static RECEIVER_WORKER: AtomicBool = AtomicBool::new(false);
///Трейт для определения какое имя будет у сериализованной структуры  
/// для автоимплементации есть макрос impl_name!(имя структуры)
pub trait PayloadType
{
    fn get_type() -> String;
}

///необходимо как то остановить основной поток после запуска иначе он выйдет из программы и все
/// # Examples
/// ```
///Server::start_server("127.0.0.1:3010");
///std::thread::sleep(Duration::from_secs(5));
///ServerSideMessage::on_receive_msg(|s, r| 
///{
///    logger::info!("Получено сообщение сервером (fn on_receive_msg) {} {:?}", s, r.payload)
///});
/// ```
pub struct Server;
impl Server
{
    pub fn start_server(host: &str)
    {
        let addr = host.to_string(); 
        ASYNC_RUNTIME.spawn(async move
        {
            debug!("Старт сервера websocket...");
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = tokio::net::TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Ошибка привязки");
            debug!("Websocet доступен на : {}", &addr);
            while let Ok((stream, _)) = listener.accept().await 
            {
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
                    if RECEIVER_WORKER.load(std::sync::atomic::Ordering::SeqCst)
                    {
                        let _ = s.unbounded_send(d);
                        debug!("Cообщение добавлено в очередь сообщений {}",s.len());
                    }
                }
                else
                {
                    logger::error!("Ошибка десериализации обьекта {:?} поступившего от клиента {} ", &data, &addr);
                }
            }
            else if msg.is_ping()
            {
                debug!("Сервером получено сообщение ping");
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


///```
///ServerSideMessage::from_str("тестовая строка от сервера").send_to_all().await;  
/// let _ = ServerSideMessage::from_struct(&test).send_to_all().await;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSideMessage
{
    ///если success false то в payload будет лежать строка с ошибкой иначе там будет объект на сонове payload_type
    pub success: bool,
    ///тип нагрузки: string | number | error | array | object | unknown | command
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

impl ServerSideMessage
{
    pub fn error(error: &str) -> Self
    {
        Self
        {
            success: false,
            payload_type:  PayloadTypeEnum::Error.to_string(),
            payload: Some(error.to_owned()),
            object_name: None
        }
    }
    pub fn get_payload_type(&self) -> PayloadTypeEnum
    {
        let pl = &self.payload_type;
        pl.into()
    }
    pub fn from_str(msg: &str) -> Self
    {
        Self
        {
            success: true,
            payload_type:  PayloadTypeEnum::String.to_string(),
            payload: Some(msg.to_owned()),
            object_name: None
        }
    }
    pub fn from_number(msg: i64) -> Self
    {
        Self
        {
            success: true,
            payload_type:  PayloadTypeEnum::Number.to_string(),
            payload: Some(msg.to_string()),
            object_name: None
        }
    }
    pub fn from_struct<T: Serialize + Clone + PayloadType>(val: &T) -> Self
    {
        let ser = serde_json::to_value(val);
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
                Self
                {
                    success: true,
                    payload_type: PayloadTypeEnum::String.to_string(),
                    payload: serde_json::to_string(val).ok(),
                    object_name: Some(T::get_type())
                }
            }
            else if obj.is_array()
            {
                Self
                {
                    success: true,
                    payload_type:  PayloadTypeEnum::Array.to_string(),
                    payload: serde_json::to_string(val).ok(),
                    object_name: Some(T::get_type())
                }
            }
            else if obj.is_string()
            {
                Self::from_str(obj.as_str().unwrap())
            }
            else if obj.is_number()
            {
                Self::from_number(obj.as_number().unwrap().as_i64().unwrap())
            }
            else 
            {
                Self::error(&format!("Тип объекта {} пока не обрабатывается", serde_json::to_string_pretty(val).unwrap()))
            }
        }
    }
    ///Если не активировать это замыкание то поступающие от клиента сообщения не будут складываться в канал, ну и обработки сообщений соотвественно не будет
    /// на случай если клиент не собирается посылать серверу сообщения и обрабатывать их не нужно
    pub fn on_receive_msg<F>(f: F ) where F: Fn(SocketAddr, ServerSideMessage) + Send + 'static
    {
        RECEIVER_WORKER.store(true, std::sync::atomic::Ordering::SeqCst);
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
    use std::time::Duration;

    use logger::{debug, error};
    use serde::{Deserialize, Serialize};

    use crate::Server;

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

    #[tokio::test]
    async fn test_server()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010");
        loop 
        {
            std::thread::sleep(Duration::from_secs(5));
        }
    }
}

