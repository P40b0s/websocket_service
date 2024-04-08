use logger::{debug, error};
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::{handshake::server::{Request, Response}, Message};
use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};
use std::net::SocketAddr;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::pin_mut;
use futures::{stream::StreamExt, future, TryStreamExt};
use crate::message::WebsocketMessage;

///Список подключенных клиентов с каналом для оправки им сообщений
static CLIENTS: Lazy<Arc<Mutex<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(Mutex::new(HashMap::new()))
});
///Канал получаемых от клиентов сообщений 
static MESSAGE_RECEIVER: Lazy<Mutex<HashMap<SocketAddr, UnboundedReceiver<WebsocketMessage>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
///Флаг что сообщения из очереди обрабатываются,  
///Cтавиться автоматически при вызове замыкания обработки сообщений
static RECEIVER_WORKER: AtomicBool = AtomicBool::new(false);


/// # Examples
/// ```
///Server::start_server("127.0.0.1:3010").await;
///std::thread::sleep(Duration::from_secs(5));
///Server::on_receive_msg(|addr, msg|
///{
///    debug!("Сервером полчено сообщение от {} через канал {}", addr, &msg.command.target);
///    
///}).await;
///loop
///{
///    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
///    let srv_wsmsg: WebsocketMessage = "test_server_cmd:test_server_method".into();
///    _ = Server::broadcast_message_to_all(&srv_wsmsg).await;
///}
/// ```
pub struct Server;
impl Server
{
    pub async fn start_server(host: &str)
    {
        let addr = host.to_string(); 
        //tokio::spawn(async move
        //{
            debug!("Старт сервера websocket...");
            // Create the event loop and TCP listener we'll accept connections on.
            let listener = tokio::net::TcpListener::bind(&addr).await;
            if let Ok(lis) = listener
            {
                debug!("Websocet доступен на : {}", &addr);
                while let Ok((stream, _)) = lis.accept().await 
                {
                    tokio::spawn(async move
                    {
                        Self::accept_connection(stream).await;
                    });
                }
            }
            else
            {
                logger::error!("Ошибка запуска сервера: {}", listener.unwrap_err().to_string())
            }
        //});
    }
    async fn add_message_receiver(socket: &SocketAddr, receiver: UnboundedReceiver<WebsocketMessage>)
    {
        let mut mr_guard = MESSAGE_RECEIVER.lock().await;
        let _ = mr_guard.insert(socket.clone(), receiver);
    }
    async fn add_message_sender(socket: &SocketAddr, sender: UnboundedSender<Message>)
    {
        let mut guard =   CLIENTS.lock().await;
        guard.insert(socket.clone(), sender);
        drop(guard);
    }

    async fn accept_connection(stream: tokio::net::TcpStream)
    {
        let (s, r) = unbounded::<WebsocketMessage>();
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
                // if let Ok(m) = msg.clone().into_text()
                // {
                //     debug!("Сервером получено сообщение: {}", m);
                // }
                
                let msg =  TryInto::<WebsocketMessage>::try_into(&msg);
                if let Ok(d) = msg
                {
                    //debug!("Сервером получено сообщение: {:?}", d);
                    if RECEIVER_WORKER.load(std::sync::atomic::Ordering::SeqCst)
                    {
                        let _ = s.unbounded_send(d);
                        //debug!("Cообщение добавлено в очередь сообщений {}",s.len());
                    }
                }
                else
                {
                    error!("Ошибка десериализации обьекта {:?} поступившего от клиента {} ", &msg.unwrap_err().to_string(), &addr);
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
        CLIENTS.lock().await.remove(&addr);
        debug!("Клиент {} отсоединен", &addr);
    }

    ///Если не активировать это замыкание то поступающие от клиента сообщения не будут складываться в канал, ну и обработки сообщений соотвественно не будет
    /// на случай если клиент не собирается посылать серверу сообщения и обрабатывать их не нужно
    pub async fn on_receive_message<F, Fut: std::future::Future<Output = ()> + Send>(f: F)
    where F:  Send + 'static + Fn(SocketAddr, WebsocketMessage) -> Fut
    {
        RECEIVER_WORKER.store(true, std::sync::atomic::Ordering::SeqCst);
        tokio::spawn(async move
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
                            f(addr.clone(), m).await;
                        }
                    }
                }
            }
        });
    }
    /// Сообщения всем подключеным клиентам
    pub async fn broadcast_message_to_all(msg: &WebsocketMessage)
    {
       
        let state = CLIENTS
        .lock()
        .await;
        let receivers = state
        .iter()
        .map(|(_, ws_sink)| ws_sink);
        let msg =  TryInto::<Message>::try_into(msg);
        if let Ok(m) = msg
        {
            for recp in receivers
            {
                recp.unbounded_send(m.clone()).unwrap();
            }
        }
        else
        {
            logger::error!("Ошибка массовой рассылки сообщения {:?}", &msg.unwrap_err().to_string());
        }
    }
    ///Сообщения всем подключеным клиентам кроме того что передан параметром addr
    pub async fn message_to_all_except_sender(addr: &SocketAddr, msg: &WebsocketMessage)
    {
        
        let state = CLIENTS
            .lock()
            .await;
            let receivers = state
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &addr)
            .map(|(_, ws_sink)| ws_sink);
        let msg =  TryInto::<Message>::try_into(msg);
        if let Ok(m) = msg
        {
            for recp in receivers
            {
                recp.unbounded_send(m.clone()).unwrap();
            }
        }
        else
        {
            logger::error!("Ошибка массовой рассылки сообщения {:?}", &msg.unwrap_err().to_string());
        }
    }
    pub async fn send(message: &WebsocketMessage, addr: &SocketAddr)
    {
       
        let msg =  TryInto::<Message>::try_into(message);
        if let Ok(m) = msg
        {
            if let Some(sender) = CLIENTS.lock().await.get(addr) 
            {
                sender.unbounded_send(m).unwrap();
            }
        }
        else
        {
            logger::error!("Ошибка отправки сообщения {:?}", &msg.unwrap_err().to_string());
        }
    }
}



#[cfg(test)]
mod tests
{
    use super::Server;

    #[tokio::test]
    async fn test_server()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010").await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
}

