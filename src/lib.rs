mod ws_service;
mod ws_message;
mod ws_client_service;
mod ws_error;
use std::{error::Error, net::SocketAddr};

use logger::debug;
pub use ws_client_service::{WsClientService, ClientRequestProcessing};
pub use ws_message::{WsServerMessage, WsClientMessage, CommandDecoder, WsErrorMessage, MessageSender};
pub use ws_service::WsService;
pub use ws_error::WSError;



///Стртует обработчик соединений websocket стартовать необходимо в рантайме tokio
/// типа из tokio::main
pub fn start_ws_service<F>(host: &str, process_client_messages : F) 
where
    F: Sync + Send + Clone + 'static + FnMut(&SocketAddr, &url::Url) -> Result<(), Box<dyn Error>>
{
    let addr = host.to_string();
    tokio::spawn(async move
    {
        debug!("Стартую websocket...");
        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = tokio::net::TcpListener::bind(&addr).await;
        let listener = try_socket.expect("Ошибка привязки");
        debug!("Websocet доступен на : {}", addr);
        while let Ok((stream, _)) = listener.accept().await 
        {
            tokio::spawn(WsService::accept_connection(stream, process_client_messages.clone()));
        }
    });
}

#[cfg(test)]
mod test
{
    use crate::{start_ws_service, ws_error::WSError};

    #[test]
    pub fn test_connection()
    {
        start_ws_service("127.0.0.1:3010", |socket, cmd|
        {
            let path = cmd.path();
            //в целом получается команда типа client:file?path=werwerwer&logs=false
            //что это команда от клиента тут мы уже знаем, дальше беерем path 
            //это часть -> file если подходит дальше обрабатываем запрос для сервиса обработки файлов, дальнейший запрос указан после file
            return Err(Box::new(WSError::ErrorParsingClientCommand("ОШИБКО!".to_owned())));
        });
    }
}