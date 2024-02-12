use logger::{debug, error, backtrace};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use std::{borrow::Cow, sync::{Arc, Mutex}, collections::HashMap, fmt::Display, error::Error};
use std::ops::ControlFlow;
use std::{net::SocketAddr, path::PathBuf};
// use tower_http::{
//     services::ServeDir,
//     trace::{DefaultMakeSpan, TraceLayer},
// };
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{pin_mut};
//allows to extract the IP of connecting user
// use axum::{extract::connect_info::ConnectInfo, Json};
// use axum::extract::ws::CloseFrame;
//allows to split the websocket stream into separate TX and RX branches
use futures::{sink::SinkExt, stream::StreamExt, future, TryStreamExt};

use crate::{ws_message::{WsErrorMessage, MessageSender}, ws_error::WSError};

use super::{WsServerMessage, WsClientService};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
// lazy_static! 
// {
//     pub static ref WS_STATE: PeerMap = PeerMap::new(Mutex::new(HashMap::new()));
// }

static WS_STATE: Lazy<PeerMap> = Lazy::new(|| {
    PeerMap::new(Mutex::new(HashMap::new()))
});

pub struct WsService;
impl WsService
{
    
    pub async fn accept_connection<F: Sync + Send + Clone + 'static + FnMut(&SocketAddr, &url::Url) -> Result<(), Box<dyn Error>>>(stream: tokio::net::TcpStream, process_client_messages : F) 
    {
        let addr = stream.peer_addr().expect("Соединение должно иметь исходящий ip адрес");
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .expect("Ошибка handsnake при извлечении данных из websocket");
        debug!("Новое websocket соединение: {}", addr);
        // Insert the write part of this peer to the peer map.
        let (tx, rx) = unbounded();
        WS_STATE.lock().unwrap().insert(addr, tx);
        let (outgoing, incoming) = ws_stream.split();
        let broadcast_incoming = incoming.try_for_each(|msg| 
        {
            if !msg.is_ping() && !msg.is_pong() && !msg.is_empty() && !msg.is_close()
            {
                let message = msg.to_text();
                if let Ok(m) = message
                {
                    debug!("Получено сообщение от {}: {}", addr, msg.to_text().unwrap());
                    let des_message = serde_json::from_str::<super::WsClientMessage>(m);
                    if des_message.is_err()
                    {
                        let des_err =  des_message.err().unwrap().to_string();
                        error!("{} -> {}",&des_err, backtrace!());
                        WsErrorMessage::err(Box::new(WSError::ErrorDeserializeClientMessage(des_err))).send(&addr);
                    }
                    else 
                    {
                        WsClientService::process_messages(addr, des_message.unwrap(), process_client_messages.clone());
                    }
                }
            }
            future::ok(())
        });
        let receive_from_others = rx.map(Ok).forward(outgoing);
        pin_mut!(broadcast_incoming, receive_from_others);
        future::select(broadcast_incoming, receive_from_others).await;
        WS_STATE.lock().unwrap().remove(&addr);
        debug!("Клиент {} отсоединен", &addr);
    }

    pub fn broadcast_all(msg: &Message)
    {
        // We want to broadcast the message to everyone except ourselves.
        let state = WS_STATE
        .lock()
        .unwrap();
        let receivers = state
        .iter()
        .map(|(_, ws_sink)| ws_sink);

        for recp in receivers
        {
            recp.unbounded_send(msg.clone()).unwrap();
        }
    }

    pub fn message_to_all_except_sender(addr: &SocketAddr, msg: &Message)
    {
        // We want to broadcast the message to everyone except ourselves.
        let state = WS_STATE
            .lock()
            .unwrap();
            let receivers = state
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &addr)
            .map(|(_, ws_sink)| ws_sink);

        for recp in receivers
        {
            let ms = msg.to_string();
            recp.unbounded_send(msg.clone()).unwrap();
        }
    }
    pub fn message_to_client<T: Serialize + Clone>(addr: &SocketAddr, msg: WsServerMessage<T>)
    {
        let msg = json!(msg);
        let msg = Message::binary(msg.to_string());
        if let Some(sender) = WS_STATE.lock().unwrap().get(addr) 
        {
            sender.unbounded_send(msg).unwrap();
        }
        else
        {
            error!("Ошибка отправки сообщения клиенту {}, возможно он уже отключен.", addr.ip().to_string())
        }
    }
    pub fn error_to_client<E: Serialize + Clone>(addr: &SocketAddr, error: E)
    {
        let msg = json!(error);
        let msg = Message::binary(msg.to_string());
        if let Some(sender) = WS_STATE.lock().unwrap().get(addr) 
        {
            sender.unbounded_send(msg).unwrap();
        }
        else
        {
            error!("Ошибка отправки сообщения клиенту {}, возможно он уже отключен.", addr.ip().to_string())
        }
    }
    pub fn message_to_all<T: Serialize + Clone>(msg: WsServerMessage<T>) 
    {
        let msg = json!(msg);
        let mm = Message::binary(msg.to_string());
        WsService::broadcast_all(&mm);
    }
    pub fn error_to_all<E: Serialize + Clone>(error: E) 
    {
        let msg = json!(error);
        let mm = Message::binary(msg.to_string());
        WsService::broadcast_all(&mm);
    }

}






//тело функции при получении собщения от клиента, убрал сюда, пусть пока побудет тут на память
//let msg = WsBusMessage::new(services::WsBusMessageType::ParseNewPacket);
            //services::MessageBus::send_message(msg);
            //let peers = WS_STATE.lock().unwrap();

            //We want to broadcast the message to everyone except ourselves.
            //let broadcast_recipients =
            //     peers.iter().filter(|(peer_addr, _)| peer_addr != &&addr).map(|(_, ws_sink)| ws_sink);

            //for recp in broadcast_recipients 
            //{
            //     recp.unbounded_send(msg.clone()).unwrap();
                
            //}
            //в зависимости от типа сообщения выбираем тип рассылки броадкаст\одиночная;