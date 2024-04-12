#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
mod client;
mod message;
mod retry;
pub use retry::retry;
pub use message::WebsocketMessage;
#[cfg(feature = "server")]
pub use server::Server;
#[cfg(feature = "client")]
pub use client::Client;
#[cfg(feature = "binary")]
pub use bitcode::{Decode, Encode};

#[cfg(test)]
mod test
{
    #[cfg(feature = "json")]
    #[derive(serde::Serialize, Debug, serde::Deserialize)]
    struct TestPayload
    {
        name: String,
        age: u32,
        characters: Vec<String>,
        is_man: bool,
        pay: u64
    }
    #[cfg(feature = "binary")]
    #[derive(bitcode::Encode, bitcode::Decode, Debug)]
    struct TestPayload
    {
        name: String,
        age: u32,
        characters: Vec<String>,
        is_man: bool,
        pay: u64
    }

    impl Default for TestPayload
    {
        fn default() -> Self 
        {
            Self 
            { 
                
                name: "Тестовое имя".to_owned(),
                age: 122,
                characters: vec![   
                                    "As text data. An unprocessed string of JSON data that you receive on an HTTP endpoint, read from a file, or prepare to send to a remote server.".to_owned(),
                                    "As an untyped or loosely typed representation. Maybe you want to check that some JSON data is valid before passing it on, but without knowing the structure of what it contains. Or you want to do very basic manipulations like insert a key in a particular spot.".to_owned(),
                                    "As a strongly typed Rust data structure. When you expect all or most of your data to conform to a particular structure and want to get real work done without JSON’s loosey-goosey nature tripping you up.".to_owned()
                                ],
                is_man: true,
                pay: 2445321124423
            }
        }
    }
    static COUNT: AtomicU32 = AtomicU32::new(0);
    use std::{net::SocketAddr, sync::atomic::AtomicU32};
    use logger::debug;
    use crate::message::WebsocketMessage;
    #[cfg(feature = "client")]
    use crate::Client;
    #[cfg(feature = "server")]
    use crate::Server;
    #[cfg(feature = "server")]
    #[cfg(feature = "client")]
    #[tokio::test]
    ///json -> 1000 итераций теста завершено за: 347.966442ms, 
    ///bin ->  1000 итераций теста завершено за: 269.315708ms, 290.960685ms, 268.72326ms, 
    pub async fn test_connection()
    {
        logger::StructLogger::initialize_logger();
        Server::start_server("127.0.0.1:3010", on_server_receive).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Client::start_client("ws://127.0.0.1:3010/", on_client_receive1).await;
        Client::start_client("ws://127.0.0.1:3010/", on_client_receive2).await;
        Client::start_client("ws://127.0.0.1:3010/", on_client_receive3).await;
        Client::start_client("ws://127.0.0.1:3010/", on_client_receive4).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let start = tokio::time::Instant::now();
        for m in 0..1000
        {
            let srv_wsmsg: WebsocketMessage = WebsocketMessage::new(m.to_string(), &TestPayload::default());
            _ = Client::send_message(&srv_wsmsg).await;
            _ = Server::broadcast_message_to_all(&srv_wsmsg).await;
        }
        let duration = start.elapsed();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        logger::info!("{} итераций теста завершено за: {:?}, ", COUNT.load(std::sync::atomic::Ordering::SeqCst), duration);
    }

    #[tokio::test]
    ///json -> 1000 итераций теста завершено за: 19.293236ms,  18.447752ms, 18.980526ms, 
    ///bin ->  1000 итераций теста завершено за: 18.908326ms, 16.522502ms,  17.019113ms, 
    pub async fn test_serialization()
    {
        logger::StructLogger::initialize_logger();
        let start = tokio::time::Instant::now();
        for m in 0..1000
        {
            let _ = WebsocketMessage::new(m.to_string(), &TestPayload::default());
        }
        let duration = start.elapsed();
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        logger::info!("1000 итераций теста сериализации завершено за: {:?}, ", duration);
    }

    #[tokio::test]
    ///json -> 1000 итераций теста завершено за: 8.069135ms, 7.929416ms, 
    ///bin ->  1000 итераций теста завершено за: 13.323196ms, 14.004534ms, 13.460934ms, 
    pub async fn test_deserialization()
    {
        logger::StructLogger::initialize_logger();
        let msg = WebsocketMessage::new("123", &TestPayload::default());
        
        let start = tokio::time::Instant::now();
        for m in 0..1000
        {
            let _ = &msg.extract_payload::<TestPayload>();
        }
        let duration = start.elapsed();
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        logger::info!("1000 итераций теста сериализации завершено за: {:?}, ", duration);
    }
    
    async fn on_server_receive(addr: SocketAddr, msg: WebsocketMessage)
    {
        //debug!("Сервером получено сообщение от клиента {} -> {}", &addr,  &msg.get_cmd());
        ()
    }
    fn on_client_receive1(msg: WebsocketMessage)
    {
        //debug!("Клиентом1 полчено сообщение {} {:?}", &msg.get_cmd(), &msg.extract_payload::<TestPayload>());
        COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        ()
    }
    fn on_client_receive2(msg: WebsocketMessage)
    {
        //debug!("Клиентом2 полчено сообщение через канал {} {:?}", &msg.get_cmd(), &msg.extract_payload::<TestPayload>());
        ()
    }
    fn on_client_receive3(msg: WebsocketMessage)
    {
        //debug!("Клиентом3 полчено сообщение через канал {} {:?}", &msg.get_cmd(), &msg.extract_payload::<TestPayload>());
        ()
    }
    fn on_client_receive4(msg: WebsocketMessage)
    {
        //debug!("Клиентом4 полчено сообщение через канал {} {:?}", &msg.get_cmd(), &msg.extract_payload::<TestPayload>());
        ()
    }
}


#[cfg(test)]
mod test_macro
{
    use websocket_derive::Contract;

    use crate::WebsocketMessage;

    #[derive(Contract)]
    pub enum Test
    {
        ///asfasfsadf
        #[contract(command="t1", is_error=true)]
        Test1,
        #[contract(command="t2")]
        Test2,
        #[contract(command="t3")]
        Test3
    }
    #[test]
    fn test()
    {
        let t1 = Test::Test1;
        println!("{}", t1.get_test());
        //let msg: WebsocketMessage = Test.into();
    }
}