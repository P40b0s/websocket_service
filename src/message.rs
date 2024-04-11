use tokio_tungstenite::tungstenite::Message;
use anyhow::{anyhow, Context, Result};


#[cfg(feature = "binary")]
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct WebsocketMessage
{
    pub success: bool,
    ///идентификатор отправления (команда) например error и тогда ошибка отправиться в поле text  
    ///или settings/tasks/update тогда в поле payload будет лежать объект task для обновления
    pub cmd: String,
    pub text: Option<String>,
    pub payload: Option<Vec<u8>>,
}

#[cfg(feature = "json")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketMessage
{
    pub success: bool,
    ///идентификатор отправления (команда) например error и тогда ошибка отправиться в поле text  
    ///или settings/tasks/update тогда в поле payload будет лежать объект task для обновления
    pub cmd: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    #[serde(default="default_payload_option")]
    #[serde(with = "b64")]
    pub payload: Option<Vec<u8>>,
}

impl WebsocketMessage
{
    ///новый экземпляр в котором нужно самостоятельно серализовать данные
    pub fn new_payload<S: ToString>(cmd: S, payload: &[u8]) -> Self
    {
        Self
        {
            success: true,
            cmd: cmd.to_string(),
            text: None,
            payload: Some(payload.to_vec())
        }
    }
    ///новый экземпляр c текстом
    pub fn new_text<S: ToString>(cmd: S, text: S) -> Self
    {
        Self
        {
            success: true,
            cmd: cmd.to_string(),
            text: Some(text.to_string()),
            payload: None
        }
    }
    ///новый экземпляр с ошибкой
    pub fn new_error<S: ToString>(cmd: S, text: S) -> Self
    {
        Self
        {
            success: false,
            cmd: cmd.to_string(),
            text: Some(text.to_string()),
            payload: None
        }
    }

    #[cfg(feature = "binary")]
    pub fn new<T: bitcode::Encode, S: ToString>(cmd: S, payload: &T) -> Self
    {
        Self
        {
            success: true,
            cmd: cmd.to_string(),
            text: None,
            payload: Some(bitcode::encode(payload))
        }
    }
    #[cfg(feature = "json")]
    pub fn new<T: serde::Serialize, S: ToString>(cmd: S, payload: &T) -> Self
    {
        let mut bytes: Vec<u8> = Vec::new();
        let vec = serde_json::to_writer(bytes, payload);
        Self
        {
            success: true,
            cmd: cmd.to_string(),
            text: None,
            payload: Some(bytes)
        }
    }
    #[cfg(feature = "json")]
    pub fn extract_payload<T>(&self) -> Result<T> where for <'de> T : serde::Deserialize<'de>
    {
        if let Some(pl) = &self.payload
        {
            let obj = serde_json::from_slice::<T>(pl).with_context(|| format!("Данный объект отличается от того который вы хотите получить"))?;
            return Ok(obj);
        }
        else 
        {
            return Err(anyhow!("В данном сообщении {} отсуствует объект для десериализации", &self.cmd));
        }
    }
    #[cfg(feature = "json")]
    pub fn to_transport_message(&self) -> Result<Message>
    {
        let mut bytes: Vec<u8> = Vec::new();
        serde_json::to_writer(bytes, self)?;
        let msg = Message::binary(bytes);
        Ok(msg)
    }
    #[cfg(feature = "binary")]
    pub fn to_transport_message(&self) -> Result<Message>
    {
        let msg = Message::binary(bitcode::encode(self));
        Ok(msg)
    }
    #[cfg(feature = "binary")]
    pub fn extract_payload<T>(&self) -> Result<T> where for <'de> T : bitcode::Decode<'de>
    {
        if let Some(pl) = &self.payload
        {
            let obj = bitcode::decode::<T>(pl).with_context(|| format!("Данный объект отличается от того который вы хотите получить"))?;
            return Ok(obj);
        }
        else
        {
            return Err(anyhow!("В данном сообщении {} отсуствует объект для десериализации", &self.cmd));
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

#[cfg(feature = "json")]
mod b64 
{
    extern crate base64;
    use serde::{Serializer, de, Deserialize, Deserializer};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        //serializer.collect_str(&base64::display::Base64Display::standard(bytes))
        if let Some(b) = bytes
        {
            serializer.serialize_str(&base64::encode(b))
        }
        else
        {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where D: Deserializer<'de>
    {
        let s = <&str>::deserialize(deserializer)?;
        let decoded = base64::decode(s);
        if let Ok(d) = decoded
        {
            Ok(Some(d))
        }
        else
        {
            return Err(de::Error::custom(decoded.err().unwrap().to_string()));
        }
    }
}