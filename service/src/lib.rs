#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
mod client;
mod retry;
pub use retry::retry;
#[cfg(feature = "server")]
pub use server::Server;
#[cfg(feature = "client")]
pub use client::Client;


#[cfg(test)]
mod test
{
    #[derive(serde::Serialize, Debug, serde::Deserialize)]
    pub struct TestPayload
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
    

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub enum TransportMessage
    {
        Test1(TestPayload),
        Test2(String)
    }
   
    pub struct Client1;
    impl Client<TransportMessage> for Client1
    {
        fn get_id() -> &'static str 
        {
            "Client1"
        }
    }
    pub struct Client2;
    impl Client<TransportMessage> for Client2
    {
        fn get_id() -> &'static str 
        {
            "Client2"
        }
    }
    pub struct Client3;
    impl Client<TransportMessage> for Client3
    {
        fn get_id() -> &'static str 
        {
            "Client3"
        }
    }
    pub struct Client4;
    impl Client<TransportMessage> for Client4
    {
        fn get_id() -> &'static str 
        {
            "Client4"
        }
    }
    pub struct WsServer;
    impl Server<TransportMessage> for WsServer{}
    static COUNT: AtomicU32 = AtomicU32::new(0);
    use std::sync::atomic::AtomicU32;
    use logger::debug;
    #[cfg(feature = "client")]
    use crate::Client;
    #[cfg(feature = "server")]
    use crate::Server;


    #[cfg(feature = "server")]
    #[cfg(feature = "client")]
    #[tokio::test]
    ///json -> 1000 итераций теста завершено за: 91.356768ms, 129.009179ms, 126.985547ms, 125.335948ms, 
    ///bin ->  1000 итераций теста завершено за: 172.269761ms, 173.672374ms, 170.545242ms, 
    /// flexbuffers -> 1000 итераций теста завершено за: 148.515492ms, 151.712027ms, 145.205206ms, 
    pub async fn test_connection()
    {
        logger::StructLogger::initialize_logger();
        WsServer::start_server("127.0.0.1:3010", |addr, msg|
        {
            async move
            {
                debug!("Сервером получено сообщение от клиента {} {:?}", &addr, &msg);
                ()
            }
        }).await;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Client1::start_client("ws://127.0.0.1:3010/", |msg: TransportMessage|
        {
            COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            debug!("Клиент получил сообщение {:?}", &msg);
           
        }).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let start = tokio::time::Instant::now();
        for _m in 0..1000
        {
            _ = Client1::send_message(TransportMessage::Test1(TestPayload::default())).await;
            _ = WsServer::broadcast_message_to_all(TransportMessage::Test2("Тестовая рассылка от сервера".to_owned())).await;
        }
        let duration = start.elapsed();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        logger::info!("{} итераций теста завершено за: {:?}, ", COUNT.load(std::sync::atomic::Ordering::SeqCst), duration);
    }

    //#[tokio::test]
    ///json -> 1000 итераций теста завершено за:  23.642501ms, 18.955071ms, 18.682914ms, 18.905361ms, 
    ///bin ->  1000 итераций теста завершено за: 20.686414ms, 19.505297ms, 18.415195ms, 17.314488ms, 17.15283ms, 17.544082ms, 
    /// flexbuffers -> 21.808916ms, 21.779735ms,  21.757869ms, 
    // pub async fn test_serialization()
    // {
    //     logger::StructLogger::initialize_logger();
    //     let start = tokio::time::Instant::now();
    //     for m in 0..1000
    //     {
    //         let _ = TransportMessage::Test1(TestPayload::default()).to_binary();
    //     }
    //     let duration = start.elapsed();
    //     tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //     logger::info!("1000 итераций теста сериализации завершено за: {:?}, ", duration);
    // }

    //#[tokio::test]
    ///json -> 1000 итераций теста завершено за: 9.710066ms, 8.600755ms,   7.954601ms, 
    ///bin ->  1000 итераций теста завершено за: 18.378167ms, 18.723982ms, 18.360396ms, 
    ///flexbuffers ->  1000 итераций теста завершено за: 15.923455ms,  11.532457ms, 13.492814ms, 
    // pub async fn test_deserialization()
    // {
    //     logger::StructLogger::initialize_logger();
    //     let msg = TransportMessage::Test1(TestPayload::default()).to_binary();
        
    //     let start = tokio::time::Instant::now();
    //     for m in 0..1000
    //     {
    //         let _ = &TransportMessage::from_binary(&msg);
    //     }
    //     let duration = start.elapsed();
    //     tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //     logger::info!("1000 итераций теста сериализации завершено за: {:?}, ", duration);
    // }
    #[cfg(feature = "server")]
    #[cfg(feature = "client")]
    #[tokio::test]
    pub async fn test_many_clients()
    {
        //поток заканчивал работу прежде чем успевал получить сообщения от клиента и сервера!
        logger::StructLogger::initialize_logger();
        tokio::spawn(async move
        {
            WsServer::start_server("127.0.0.1:3010", |addr, _msg|
            {
                async move
                {
                    logger::debug!("Сервером получено сообщение от клиента {} ", &addr);
                    ()
                }
            }).await;
            Client1::start_client("ws://127.0.0.1:3010/", |msg: TransportMessage|
            {
                debug!("Клиент получил сообщение {:?}", &msg);
            }).await;
            Client2::start_client("ws://127.0.0.1:3010/", |msg: TransportMessage|
            {
                debug!("Клиент 2 получил сообщение {:?}", &msg);
            }).await;
            for _m in 0..10
            {
                _ = Client1::send_message(TransportMessage::Test1(TestPayload::default())).await;
                _ = Client2::send_message(TransportMessage::Test1(TestPayload::default())).await;
                _ = WsServer::broadcast_message_to_all(TransportMessage::Test2("Тестовая рассылка от сервера".to_owned())).await;
            }
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    // async fn on_server_receive(addr: SocketAddr, msg: TransportMessage)
    // {
       
    //     debug!("Сервером получено сообщение от клиента {} -> {:?}", &addr, msg);
    //     ()
    // }
    // fn on_client_receive1(msg: TransportMessage)
    // {
    //     // let mut string = Option::<String>::None;
    //     // let mut  object = Option::<TestPayload>::None;
    //     // match msg
    //     // {
    //     //     TransportMessage::Test1(p) => object = Some(p),
    //     //     TransportMessage::Test2(s) => string = Some(s)
    //     // };
    //     // debug!("Клиентом1 полчено сообщение {:?} или {:?}", string, object);
    //     COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    //     ()
    // }
    // fn on_client_receive2(msg: TransportMessage)
    // {
    //     debug!("Клиентом1 полчено сообщение {:?}", &msg);
    //     ()
    // }
    // fn on_client_receive3(msg: TransportMessage)
    // {
    //     debug!("Клиентом2 полчено сообщение {:?}", &msg);
    //     ()
    // }
    // fn on_client_receive4(msg: TransportMessage)
    // {
    //     debug!("Клиентом3 полчено сообщение {:?}", &msg);
    //     ()
    // }
}


// #[cfg(test)]
// mod test_macro
// {
//     use websocket_derive::Contract;

//     use crate::WebsocketMessage;

//     #[derive(Contract)]
//     pub enum Test
//     {
//         ///asfasfsadf
//         #[contract(command="t1", is_error=true)]
//         Test1(String),
//         #[contract(command="t2")]
//         Test2,
//         #[contract(command="t3")]
//         Test3
//     }

//     enum TT
//     {
//         Test1(String, String)
//     }
//     #[test]
//     fn test()
//     {
//         let t1 = Test::Test1("123".to_owned());
//         //let t1 = Test::Test1;
//         t1.get_test();
//         //let msg: WebsocketMessage = Test.into();
//     }
// }