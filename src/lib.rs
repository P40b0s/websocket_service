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
    #[derive(Serialize, Debug, Deserialize)]
    struct TestPayload
    {
        name: String
    }
    use std::net::SocketAddr;

    use logger::debug;
    use serde::{Deserialize, Serialize};
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
        Server::start_server("127.0.0.1:3010", on_server_receive).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Client::start_client("ws://127.0.0.1:3010/", on_client_receive).await;
        //client.on_receive_message(on_client_receive).await;
        //Server::on_receive_message(on_server_receive).await;
        loop
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let cli_wsmsg: WebsocketMessage = "test_client_cmd:test_client_method".into();
            let srv_wsmsg: WebsocketMessage = "test_server_cmd:test_server_method".into();
            let srv_wsmsg2: WebsocketMessage = WebsocketMessage::new_with_flex_serialize("with_payload1", "test", Some(&TestPayload{name: "TEST".to_owned()}));
            _ = Client::send_message(&cli_wsmsg).await;
            _ = Server::broadcast_message_to_all(&srv_wsmsg).await;
            _ = Server::broadcast_message_to_all(&srv_wsmsg2).await;
        }
    }
    
    async fn on_server_receive(addr: SocketAddr, msg: WebsocketMessage)
    {
        debug!("Сервером получено сообщение от клиента {} через канал {}", &addr,  &msg.command.target);
        ()
    }
    fn on_client_receive(msg: WebsocketMessage)
    {
        debug!("Клиентом полчено сообщение через канал {} {:?}", &msg.command.target, &msg.command.payload);
        ()
    }
}