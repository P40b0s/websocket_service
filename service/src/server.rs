use logger::{debug, error, info};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::{handshake::server::{Request, Response}, Message};
use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};
use std::net::SocketAddr;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::pin_mut;
use futures::{future::{self, Either}, stream::{ select_all, StreamExt}, FutureExt, SinkExt, TryFutureExt, TryStreamExt};
use crate::{message::WebsocketMessage, retry};

///Список подключенных клиентов с каналом для оправки им сообщений
static CLIENTS: Lazy<Arc<RwLock<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(RwLock::new(HashMap::new()))
});


/// # Examples
/// ```
///Server::start_server("127.0.0.1:3010", on_server_receive).await;
///async fn on_server_receive(addr: SocketAddr, msg: WebsocketMessage)
///{
///    debug!("Сервером получено сообщение от клиента {} через канал {}", &addr,  &msg.command.target);
///    ()
///}
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
    pub async fn start_server<F, Fut: std::future::Future<Output = ()> + Send>(host: &str, f: F)
    where F:  Send + Sync+ 'static + Copy + Fn(SocketAddr, WebsocketMessage) -> Fut
    {
        let addr = host.to_string();
        tokio::spawn(async move
        {
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
                        Self::accept_connection(stream, f).await;
                    });
                }
            }
            else
            {
                logger::error!("Ошибка запуска сервера: {}", listener.unwrap_err().to_string())
            }
        });
    }

    async fn add_message_sender(socket: &SocketAddr) -> UnboundedReceiver<Message>
    {
        let (sender, receiver) = unbounded();
        let mut guard =   CLIENTS.write().await;
        guard.insert(socket.clone(), sender);
        drop(guard);
        receiver
    }

    async fn accept_connection<F, Fut: std::future::Future<Output = ()> + Send>(stream: tokio::net::TcpStream, f:F)
    where F:  Send + Copy + 'static + Fn(SocketAddr, WebsocketMessage) -> Fut
    {
        let addr = stream.peer_addr().expect("Соединение должно иметь исходящий ip адрес");
        // let headers_callback = |req: &Request, mut response: Response| 
        // {
        //     debug!("Получен новый ws handshake от {}", &addr);
        //     debug!("Путь запроса: {}", req.uri().path());
        //     debug!("Хэдеры запроса:");
        //     for (ref header, _value) in req.headers() 
        //     {
        //         debug!("* {}: {:?}", header, _value);
        //     }
        //     Ok(response)
        // };
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .expect("Ошибка handsnake при извлечении данных из websocket");
        let receiver = Self::add_message_sender(&addr).await;
        let (outgoing, incoming) = ws_stream.split();
       
        let from_ws = incoming.try_for_each(|msg| 
        {
            if !msg.is_ping() && !msg.is_pong() && !msg.is_empty() && !msg.is_close()
            {
                let msg =  WebsocketMessage::from_transport_message(&msg);
                if let Ok(d) = msg
                {
                    tokio::spawn(async move 
                    {
                        f(addr.clone(), d).await;
                    });
                }
                else
                {
                    error!("Ошибка десериализации обьекта {:?} поступившего от клиента {} ", &msg.unwrap_err().to_string(), &addr);
                }
            }
            else if msg.is_ping()
            {
                debug!("Сервером получено сообщение ping {}", &addr);
            }
            else if msg.is_close()
            {
                tokio::task::spawn(async move
                {
                    debug!("Сервером получено сообщение is_close {}", &addr);
                    let mut guard = CLIENTS.write().await;
                    guard.remove(&addr.clone());
                    drop(guard);
                });
            }
            future::ok(())
        });
        //let tt = tokio::spawn(receiver.map(Ok).forward(outgoing));
        let send_to_ws = receiver.map(Ok).forward(outgoing);
        pin_mut!(from_ws);
        let _ = future::select(from_ws, send_to_ws).await;
        let mut guard = CLIENTS.write().await;
        guard.remove(&addr);
        drop(guard);
        debug!("Клиент {} отсоединен", &addr);
    }
    /// Сообщения всем подключеным клиентам
    pub async fn broadcast_message_to_all(msg: &WebsocketMessage)
    {
        let state = CLIENTS.read().await;
        //debug!("Отправка сообщений {} клиентам", state.len());
        let msg =   msg.to_transport_message();
        if let Ok(m) = msg
        {
            for (_, sender) in state.iter()
            {
                if let Err(err) = sender.unbounded_send(m.clone())
                {
                    error!("{:?}", err);
                }
                //else 
                //{
                //    debug!("Сообщение отправлено {}, сообщений в канале: {}", addr, sender.len());
                //}
            }
        }
        else
        {
            logger::error!("Ошибка массовой рассылки сообщения {:?}", &msg.unwrap_err().to_string());
        }
    }
    ///Сообщения всем подключеным клиентам кроме того что передан параметром addr
    pub async fn message_to_all_except_sender(sender_addr: &SocketAddr, msg: &WebsocketMessage)
    {
        let state = CLIENTS
            .read()
            .await;
        let msg =  msg.to_transport_message();
        if let Ok(m) = msg
        {
            for (addr, sender) in state.iter()
            {
                if sender_addr != addr
                {
                    if let Err(err) = sender.unbounded_send(m.clone())
                    {
                        error!("{:?}", err);
                    }
                }
            }
        }
        else
        {
            logger::error!("Ошибка массовой рассылки сообщения {:?}", &msg.unwrap_err().to_string());
        }
    }
    pub async fn send(message: &WebsocketMessage, target_addr: &SocketAddr)
    {
        let msg =  message.to_transport_message();
        if let Ok(m) = msg
        {
            if let Some(sender) = CLIENTS.read().await.get(target_addr)
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
    use std::net::SocketAddr;
    use logger::debug;
    use crate::WebsocketMessage;
    use super::Server;

    #[tokio::test]
    async fn test_server()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010", receiver).await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn receiver(addr: SocketAddr, wsmsg: WebsocketMessage)
    {
        debug!("сообщение от клиента {} {:?}", addr, wsmsg)
    }
}

