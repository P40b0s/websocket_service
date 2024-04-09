use std::{collections::HashMap, net::{SocketAddr, TcpListener}, sync::Arc};

use futures::{future, pin_mut};
use logger::{debug, error, info};
use once_cell::sync::Lazy;
use tokio::{spawn, sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, RwLock}};
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response}, Message,
};

use crate::WebsocketMessage;

static CLIENTS: Lazy<Arc<RwLock<HashMap<SocketAddr, UnboundedSender<Message>>>>> = Lazy::new(|| 
{
    Arc::new(RwLock::new(HashMap::new()))
});

async fn add_message_sender(socket: &SocketAddr, sender: UnboundedSender<Message>)
{
    let mut guard =   CLIENTS.write().await;
    guard.insert(socket.clone(), sender);
    drop(guard);
}
pub async fn start_server()
{
    let server = TcpListener::bind("127.0.0.1:3010").expect("Ошибка запуска сервера, возможно такой адрес уже используется");
    for stream in server.incoming()
    {
        spawn(async move 
        {
            let stream = stream.unwrap();
            let addr = stream.peer_addr().expect("Соединение должно иметь исходящий ip адрес");
            let callback = |req: &Request, mut response: Response| 
            {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().path());
                println!("The request's headers are:");
                for (ref header, _value) in req.headers() 
                {
                    println!("* {}", header);
                }

                // Let's add an additional header to our response to the client.
                let headers = response.headers_mut();
                headers.append("MyCustomHeader", ":)".parse().unwrap());
                headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                Ok(response)
            };
            let mut websocket = accept_hdr(stream, callback).unwrap();
            let (sender, mut receiver) = unbounded_channel();
            add_message_sender(&addr, sender).await;
            loop 
            {
                if let Some(r) = receiver.recv().await
                {
                    websocket.send(r).unwrap();
                }
                if let Ok(msg) = websocket.read()
                {
                    if msg.is_binary()
                    {
                        let msg =  TryInto::<WebsocketMessage>::try_into(&msg);
                        if let Ok(d) = msg
                        {
                            info!("От клиента поступило сообщение! {:?}", d);
                        }
                        else
                        {
                            error!("Ошибка десериализации обьекта {:?} поступившего от клиента {} ", &msg.unwrap_err().to_string(), &addr);
                        }
                    }
                }
            }
        });
    }
}


pub async fn broadcast_message_to_all(msg: &WebsocketMessage)
{
    let state = CLIENTS.read().await;
    // let receivers = state
    // .iter()
    // .map(|(_, ws_sink)| ws_sink);
    debug!("Отправка сообщений {} клиентам", state.len());
    let msg =  TryInto::<Message>::try_into(msg);
    if let Ok(m) = msg
    {
        for (addr, sender) in state.iter()
        {
            if let Err(err) = sender.send(m.clone())
            {
                error!("{:?}", err);
            }
            else 
            {
                info!("Сообщение отправлено {}", addr);
            }
        }
    }
    else
    {
        logger::error!("Ошибка массовой рассылки сообщения {:?}", &msg.unwrap_err().to_string());
    }
}


#[cfg(test)]
mod test
{
    use logger::debug;
    use serde::{Deserialize, Serialize};
    use crate::WebsocketMessage;

    #[derive(Serialize, Debug, Deserialize)]
    struct TestPayload
    {
        name: String
    }
    use super::broadcast_message_to_all;
    #[tokio::test]
    async fn test_all()
    {
        logger::StructLogger::initialize_logger();
        tokio::spawn(async
        {
            super::start_server().await;
        });
        debug!("server is starting");
       
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
            tokio::task::spawn_blocking(||
                {
                    super::super::sync_client::start_client();
                }
            ).await;
           
        
        debug!("client is starting");
        
        loop 
        {
            let srv_wsmsg: WebsocketMessage = "test_server_cmd:test_server_method".into();
            let srv_wsmsg2: WebsocketMessage = WebsocketMessage::new_with_flex_serialize("with_payload1", "test", Some(&TestPayload{name: "TEST".to_owned()}));
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            broadcast_message_to_all(&srv_wsmsg2).await;
        }
    }
}