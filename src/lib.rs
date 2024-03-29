mod server;
mod client;
pub mod macros;
mod message;

use std::fmt::Display;

use serde::{Deserialize, Serialize};
pub use server::{Server, PayloadType};
pub use client::{start_client};


#[cfg(test)]
mod test
{
    use std::time::Duration; 
    use serde::{Deserialize, Serialize};

    use crate::{client::{start_client, ClientMessage}, impl_name, message::WebsocketMessage, server::{PayloadType, Server}};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TestStruct
    {
        pub success: bool,
        pub age: u32,
        pub legacy: Option<String>
    }
    impl_name!(TestStruct);

    #[tokio::test]
    pub async fn test_connection()
    {
        logger::StructLogger::initialize_logger();
        tokio::spawn(async
        {
            Server::start_server("127.0.0.1:3010").await;
        });
       
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        tokio::spawn(async
        {
            start_client("ws://127.0.0.1:3010/").await;
        });
            
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let wsmsg: WebsocketMessage = "test_cmd:test_method".into();
             _ = wsmsg.send_message().await;
            //let _ = ServerSideMessage::from_struct(&test).send_to_all().await;
            //let _ = ClientSideMessage::from_str("тестовая строка от клиента").send().await;
        }
    }
}