use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command
{
    pub target: String,
    pub method: String,
    #[serde(skip_serializing_if="Option::is_none")]
    #[serde(default="default_args_option")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    #[serde(default="default_payload_option")]
    pub payload: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketMessage
{
    pub success: bool,
    pub command: Command,
}
impl WebsocketMessage
{
    pub fn new(target: &str, method: &str, payload: Option<&[u8]>) -> Self
    {
        Self
        {
            success: true,
            command: Command 
            { 
                target: target.to_owned(),
                method: method.to_owned(),
                args: None,
                payload: payload.and_then(|a| Some(a.to_vec())) 
            }
        }
    }
}

fn default_payload_option() -> Option<Vec<u8>>
{
    None
}
fn default_args_option() -> Option<Vec<String>>
{
    None
}

impl From<&str> for WebsocketMessage
{
    fn from(value: &str) -> Self 
    {
        if let Some(sp) = value.split_once(':')
        {
            Self
            {
                success: true,
                command: Command 
                { 
                    target: sp.0.to_owned(), 
                    method: sp.1.to_owned(), 
                    args: None, 
                    payload: None 
                }
            }
        }
        else 
        {
            Self
            {
                
                success: true,
                command: Command 
                { 
                    target: value.to_owned(), 
                    method: "".to_owned(), 
                    args: None, 
                    payload: None 
                }
            }
        }
    }
}



impl TryFrom<&WebsocketMessage> for Message
{
    type Error = String;
    fn try_from(value: &WebsocketMessage) -> Result<Self, Self::Error> 
    {
        let mut s = flexbuffers::FlexbufferSerializer::new();
        let _ = value.serialize(&mut s).map_err(|e| e.to_string());
        let obj = s.view();
        let msg = Message::binary(obj);
        Ok(msg)
   }
}

impl TryFrom<&Message> for WebsocketMessage
{
    type Error = String;
    fn try_from(value: &Message) -> Result<Self, Self::Error> 
    {
        if !value.is_binary()
        {
            let err = "Поступившее сообщение не содержит бинарных данных".to_string();
            logger::error!("{}", &err);
            return Err(err);
        }
        let data = value.to_owned().into_data();
        let r = flexbuffers::Reader::get_root(data.as_slice()).unwrap();
        let deserialize = WebsocketMessage::deserialize(r);
        if let Ok(d) = deserialize
        {
            return Ok(d);
        }
        else
        {
            let err = format!("Ошибка десериализации обьекта {:?} поступившего от клиента", &data);
            logger::error!("{}", &err);
            return Err(err);
        }
    }
}