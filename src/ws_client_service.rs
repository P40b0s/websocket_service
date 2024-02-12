use std::{net::SocketAddr, error::Error};

use logger::{debug, error};
use serde::{Serialize, Deserialize};

use crate::{WsServerMessage, WsService, ws_message::{WsClientMessage, WsErrorMessage, MessageSender}, ws_error::WSError};


pub trait ClientRequestProcessing<'a>
{
    fn process_request(addr: &'a SocketAddr, command: &'a url::Url) -> Result<(), Box<dyn Error>>;
    //fn decode_command(command: &'b url::Url) -> Result<T, WebsocketServiceError>;
}
///ПОка парсим только текстовые сообщения от клиентов
pub struct WsClientService;
pub const CLIENT_SCHEME: &str = "client";
impl WsClientService
{
    pub fn process_messages<F: Sync + Send + Clone + FnMut(&SocketAddr, &url::Url) -> Result<(), Box<dyn Error>>>(addr: SocketAddr, message: WsClientMessage, mut process_client_messages: F)
    {
        let command = message.get_command();
        if let Ok(cmd) = command
        {
            let scheme = cmd.scheme();
            if scheme != CLIENT_SCHEME
            {
                spawn_error(&addr, Box::new(WSError::ErrorCommandScheme(scheme.to_owned())));
                return;
            }
            let prc = process_client_messages(&addr, &cmd);
            if prc.is_err()
            {
                let e = prc.err().unwrap();
                spawn_error(&addr, e);
            }
        }
        else 
        {
            spawn_error(&addr, Box::new(command.err().unwrap())); 
        }
    }

}

fn spawn_error(addr: &SocketAddr, error: Box<dyn Error>)
{
    error!("{}", error.to_string());
    WsErrorMessage::err(error).send(addr);
}

