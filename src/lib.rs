#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
mod client;
mod message;
pub use message::{WebsocketMessage, Command};
#[cfg(feature = "server")]
pub use server::Server;
#[cfg(feature = "client")]
pub use client::Client;


#[cfg(test)]
mod test
{
    use logger::debug;
    use crate::message::WebsocketMessage;
    #[cfg(feature = "client")]
    use crate::Client;
    #[cfg(feature = "server")]
    use crate::Server;
    #[cfg(feature = "server")]
    #[cfg(feature = "client")]
    #[tokio::test]
    pub async fn test_connection()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010").await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Client::start_client("ws://127.0.0.1:3010/").await;
        let _ = Client::on_receive_message(|msg|
        {
            debug!("Клиентом полчено сообщение через канал {}", &msg.command.target);
        }).await;
        Server::on_receive_msg(|addr, msg|
        {
            debug!("Сервером полчено сообщение от {} через канал {}", addr, &msg.command.target);
        }).await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let cli_wsmsg: WebsocketMessage = "test_client_cmd:test_client_method".into();
            let srv_wsmsg: WebsocketMessage = "test_server_cmd:test_server_method".into();
             _ = Client::send_message(&cli_wsmsg).await;
             _ = Server::broadcast_message_to_all(&srv_wsmsg).await;
        }
    }
}