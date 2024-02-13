mod server;
mod client;
use std::{error::Error, net::SocketAddr};

use logger::debug;
use once_cell::sync::{Lazy, OnceCell};
use tokio::runtime::Runtime;
pub use server::{ServerSideMessage, Server};



// static ASYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

// ///Стртует обработчик соединений websocket стартовать необходимо в рантайме tokio
// /// типа из tokio::main
// pub fn start_ws_service(host: &str)
// {
//     let addr = host.to_string(); 
//     ASYNC_RUNTIME.spawn(async move
//     {
//         debug!("Стартую websocket...");
//         // Create the event loop and TCP listener we'll accept connections on.
//         let try_socket = tokio::net::TcpListener::bind(&addr).await;
//         let listener = try_socket.expect("Ошибка привязки");
//         debug!("Websocet доступен на : {}", &addr);
//         while let Ok((stream, _)) = listener.accept().await 
//         {
//             debug!("НОВОЕ СОЕДИНЕНИЕ!");
//             WsService::accept_connection(stream).await;
//         }
//     });
// }

#[cfg(test)]
mod test
{
    use std::time::Duration;

    use logger::debug;
    use tokio::runtime::Runtime;

    use crate::{client::{start_client, ClientSideMessage}, server::{ServerSideMessage, Server}};

    #[tokio::test]
    pub async fn test_connection()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010");
        std::thread::sleep(Duration::from_secs(5));
        start_client("ws://127.0.0.1:3010/", |message|
        {
            logger::info!("Клиентом получено новое сообщение {:?}", message.payload);
        });
        ServerSideMessage::on_receive_msg(|s, r| 
        {
            logger::info!("Получено сообщение сервером (fn on_receive_msg) {} {:?}", s, r.payload)
        });
            
        
        loop 
        {
            std::thread::sleep(Duration::from_secs(5));
            let server_msg = ServerSideMessage::from_str("тестовая строка от сервера");
            let client_msg = ClientSideMessage::from_str("тестовая строка от клиента");
            let _ = server_msg.send_to_all().await;
            let _ = client_msg.send().await;

           
           
        }
    }
}