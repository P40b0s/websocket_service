use std::{fmt::{self, Display}, net::SocketAddr, error::Error};

use logger::debug;
use serde::{Serialize, Deserialize, de::{Visitor, MapAccess, value::BoolDeserializer}, Deserializer};
use serde_json::{Value, Map};
use url::{Url, form_urlencoded::Parse, ParseError};
use crate::ws_error::WSError;

use super::{WsService};


#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WsServerMessage<T: Serialize + Clone>
{
    pub success: bool,
    pub info: String,
    pub command: String,
    pub payload: Option<T>
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WsErrorMessage
{
    pub success: bool,
    pub info: String,
    pub command: String,
}

impl WsErrorMessage
{
    pub fn err(error: Box<dyn Error>) -> Self
    {
        WsErrorMessage
        {
            success: false,
            info: error.to_string(),
            command: "server:error".to_owned(),
        }
    }
    pub fn err_custom(error: &str) -> Self
    {
        WsErrorMessage
        {
            success: false,
            info: error.to_owned(),
            command: "server:error".to_owned(),
        }
    }
}
impl MessageSender for WsErrorMessage
{
    fn send(&self, addr: &SocketAddr)
    {
        WsService::error_to_client(addr, self.clone());
    }
    fn send_to_all(&self)
    {
        WsService::error_to_all(self.clone());
    }
}

impl<T: Serialize + Clone> Default for WsServerMessage<T>
{
    fn default() -> Self 
    {
        WsServerMessage
        { 
            success: false,
            info: "".to_owned(),
            command: "server:none".to_owned(),
            payload: None 
        }
    }
}

impl<T: Serialize + Clone> WsServerMessage<T>
{
    pub fn get_payload(&self) -> &Option<T>
    {
        &self.payload
    }
   
    pub fn new<C: Display>(obj: T, command: C) -> WsServerMessage<T>
    {
        WsServerMessage
        {
            success: true,
            info: "".to_owned(),
            command: command.to_string(),
            payload: Some(obj)
        }
    }
    pub fn new_with_message<C: Display>(obj: T, command: C, info: &str) -> WsServerMessage<T>
    {
        WsServerMessage
        {
            success: true,
            info: info.to_owned(),
            command: command.to_string(),
            payload: Some(obj)
        }
    }
}

impl<T: Serialize + Clone> MessageSender for WsServerMessage<T>
{
    fn send(&self, addr: &SocketAddr)
    {
        WsService::message_to_client(addr, self.clone());
    }
    fn send_to_all(&self)
    {
        WsService::message_to_all(self.clone());
    }
}

pub trait CommandDecoder<'a, T, E: Error>
{
    ///Пример декодирования запроса в url:
    ///client:pdf?thumbnails=false&page=4&path=234234/00000.pdf
    fn decode(command: &'a url::Url) -> Result<T, E>;
}
///Тейт для отправки сообщений, сделал потому что необходимо имплементировать
///функционал как в отправку сообщейний так и в отправку ошибок
pub trait MessageSender
{
    fn send(&self, addr: &SocketAddr);
    fn send_to_all(&self);
}

///Сообщение от клиента, мы заранее незнаем какой тип будет у payload
/// поэтому принимаем ее строкой и десериализуем если это необходимо с помощью хм...
/// пока об этом не думал, ножно на основе какой то спец команды
/// client:object?тип_структуры
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WsClientMessage
{
    pub success: bool,
    pub info: String,
    pub command: String,
    pub payload: Option<String>
}

impl WsClientMessage
{
    pub fn get_command(&self) -> Result<url::Url, WSError>
    {
        url::Url::parse(&self.command).map_err(|op|op.into())
        
    }
    pub fn get_native_command(&self) -> &str
    {
        &self.command
    }
}






//paths server:pdf?status
// impl Display for WsCommand
// {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
//     {
//         match self
//         {
//             WsCommand::ClientGetSenders => Url::
//         }
        
//     }
// }


// impl From<&str> for WsMessagePayload
// {
//     fn from(value: &str) -> Self 
//     {
//         match value
//         {
//             "get_senders" => WsMessagePayload::GetSenders,
//             "get_packets" => WsMessagePayload::GetPackets,
//             "parser_status" => WsMessagePayload::ParserStatus,
//             "new_packets_found" => WsMessagePayload::NewPacketsFound,
//             "pdf_request" => WsMessagePayload::PdfRequest,
//             "pdf_response" => WsMessagePayload::PdfResponse,
//             "pdf_processing_status" => WsMessagePayload::PdfProcessingStatus,
//             "error" => WsMessagePayload::Error,
//             _ => WsMessagePayload::NotSupport
//         }
//     }
// }

// impl From<&WsMessagePayload> for String
// {
//     fn from(value: &WsMessagePayload) -> Self 
//     {
//         match value
//         {
//             WsMessagePayload::ServerNewPacketsFound(_) => "server:new_packets_found".to_owned(),
//             WsMessagePayload::ServerPdfResponse(_) => "server:pdf_response".to_owned(),
//             WsMessagePayload::ServerPdfProcessingStatus(_) => "server:pdf_processing_status".to_owned(),
//             WsMessagePayload::NotSupport => "команда не поддерживается".to_owned(),
//             WsMessagePayload::Error => "error".to_owned(),
//             WsMessagePayload::None => "none".to_owned()
//         }
//     }
// }
// impl From<WsMessagePayload> for String
// {
//     fn from(value: WsMessagePayload) -> Self 
//     {
//         let v = &value;
//         let str:String = v.into();
//         str
//     }
// }

// impl ToString for WsMessagePayload
// {
//     fn to_string(&self) -> String 
//     {
//         self.into()
//     }
// }












#[cfg(test)]
mod tests
{
    use logger::{debug, error};


    use super::{WsServerMessage, WsClientMessage};

    #[test]
    fn test_deserialize_message()
    {
        logger::StructLogger::initialize_logger();
        let msg_with_payload = "{\"success\":true,\"info\":\"\",\"command\":\"pdf_request\",\"payload\":\"{\\\"thumbnails\\\":true,\\\"localPath\\\":\\\"15933154/text0000000000.pdf\\\",\\\"page\\\":1}\"}";
        let msg = "{\"success\":true,\"info\":\"\",\"command\":\"pdf_request\"}" ;
        let message = serde_json::from_str::<WsClientMessage>(msg_with_payload);
        if message.is_err()
        {
            error!("{}", message.err().unwrap());
        }
        else
        {
            debug!("{:?}", message.unwrap());
        }
    }

    #[test]
    fn test_deserialize_message_by_url()
    {
        logger::StructLogger::initialize_logger();
        let msg = "server:pdf?thumb=false&page=2&path=15933154/text0000000000.pdf";
        let url = url::Url::parse(msg).unwrap();
        let scheme = url.scheme();
        let path = url.path();
        let query = url.query_pairs();
        for q in query.into_iter()
        {
            debug!("{} = {}", q.0, q.1);
        }
    }
}





//Оставил для теста!

// impl<'de> Deserialize<'de> for WsMessage
// {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: Deserializer<'de>
//     {
//         //deserializer.deserialize_any(CustomVisitor)
//         deserializer.deserialize_map(WsMessageVisitor)
//     }
// }

// struct WsMessageVisitor;
// impl<'de> Visitor<'de> for WsMessageVisitor 
// {
//     type Value = WsMessage;
//     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         write!(formatter, "Необходим валидный json для обмена сообщениями через websocket")
//     }
    
//     fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
//     where
//         M: MapAccess<'de>
//     {
//         let mut success = None;
//         let mut info = None;
//         let mut command: Option<String> = None;
//         let mut payload: Option<String> = None;
//         let mut error: Option<M::Error> = None;
//         let mut ws_msg: WsMessagePayload = WsMessagePayload::NotSupport;

//         while let Some(k) = map.next_key::<&str>()? 
//         {
//             if k == "success" 
//             {
//                 success = Some(map.next_value()?);
//             }
//             else if k == "info" 
//             {
//                 info = Some(map.next_value()?);
//             }
//             else if k == "command" 
//             {
//                 command = Some(map.next_value()?);
//             }
//             else if k == "payload" 
//             {
//                 payload = Some(map.next_value()?);
//             }
//             else 
//             {
//                 return Err(serde::de::Error::custom(&format!("Неверный ключ: {}", k)));
//             }
//         }
//         if command.is_none()
//         {
//             return Err(serde::de::Error::custom("Отсутсвует команда -> command"));
//         }
//         let cmd = command.as_ref().unwrap();
//         let parsed_cmd = url::Url::parse(cmd);
//         if let Ok(cmd) = parsed_cmd
//         {
//             let server_scheme = "server";
//             let scheme = cmd.scheme();
//             if scheme != server_scheme
//             {
//                 return Err(serde::de::Error::custom(&format!("Неправильная схема управляющей команды {} получена схема: {}, сервером принимается только {}", cmd, scheme, server_scheme)));
//             }
//             let path  = cmd.path();
//             match path
//             {
//                 "pdf" => 
//                 {
//                     let req = process_pdf_request_command(cmd.query_pairs());
//                     if let Ok(ms) = req
//                     {
//                         ws_msg = ms;
//                     }
//                     else
//                     {
//                         error = Some(serde::de::Error::custom(req.err().unwrap()));
//                     }
//                 },
//                 _ =>
//                 {
//                   ws_msg =  WsMessagePayload::NotSupport;
//                   error = Some(serde::de::Error::custom(["Команда ", cmd, " не поддерживается сервером"].concat()));
//                 } 
//             }
//         }
//         else
//         {
//             return Err(serde::de::Error::custom(&format!("Не распознана управляющая команда: {}", cmd)));
//         }
//         if error.is_some()
//         {
//             return Err(error.unwrap());
//         }
//         let msg = WsMessage 
//         {
//             success: success.unwrap(),
//             info: info.unwrap(),
//             command: cmd.to_owned(),
//             payload: Some(ws_msg)
//         };
//         Ok(msg)
//     }
// }