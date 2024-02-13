mod server;
mod client;
pub use server::{ServerSideMessage, Server};
pub use client::{start_client, ClientSideMessage};

#[cfg(test)]
mod test
{
    use std::time::Duration; 
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