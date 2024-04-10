use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use anyhow::{anyhow, bail, Context, Result};

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
impl Command
{
    pub fn get_target(&self) -> &str
    {
        &self.target
    }
    pub fn get_method(&self) -> &str
    {
        &self.method
    }
    ///извлечь нагрузку из текущего сообщения
    pub fn extract_payload<T>(&self) -> Result<T> where for <'de> T : Deserialize<'de>
    {
        if let Some(pl) = &self.payload
        {
            let r = flexbuffers::Reader::get_root(pl.as_slice())?;
            let deserialize = T::deserialize(r).with_context(|| format!("Данный объект отличается от того который вы хотите получить"))?;
            return Ok(deserialize);
        }
        else 
        {
            return Err(anyhow!("В данном сообщении {}:{} отсуствует объект для десериализации", &self.target, &self.method));
        }
    }

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
    ///новый экземпляр в котором нужно самостоятельно серализовать данные с помощью схемы flexbuffers
    pub fn new<S: ToString>(target: S, method: S, payload: Option<&[u8]>) -> Self
    {
        Self
        {
            success: true,
            command: Command 
            { 
                target: target.to_string(),
                method: method.to_string(),
                args: None,
                payload: payload.and_then(|a| Some(a.to_vec())) 
            }
        }
    }
    ///Новый экземпляр с сериализаций fexbuffers
    pub fn new_with_flex_serialize<T: Serialize, S: ToString>(target: S, method: S, payload: Option<&T>) -> Self
    {
        let payload = payload.and_then(|pl|
        {
            let mut s = flexbuffers::FlexbufferSerializer::new();
            let _ = pl.serialize(&mut s).map_err(|e| e.to_string());
            Some(s.view().to_vec())
        });
        Self
        {
            success: true,
            command: Command 
            { 
                target: target.to_string(),
                method: method.to_string(),
                args: None,
                payload
            }
        }
    }
    ///добавить аргументы к текущей команде
    pub fn add_args(mut self, args: &[String]) -> Self
    {
        self.command.args.as_mut().and_then(|a| Some(a.extend_from_slice(args)));
        self
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
        let _ = value.serialize(&mut s).map_err(|e| e.to_string())?;
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