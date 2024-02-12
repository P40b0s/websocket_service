use std::error::Error;

#[derive(Debug)]
pub enum WSError
{
    ErrorParsingClientCommand(String),
    ErrorCommandScheme(String),
    ErrorDeserializeClientMessage(String)
}
impl Error for WSError {}
impl std::fmt::Display for WSError
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result 
    {
        match self 
        {
            WSError::ErrorParsingClientCommand(c) => 
            {
                let err = ["Ошибка парсинга: ", c].concat();
                fmt.write_str(&err)
            }
            WSError::ErrorCommandScheme(c) => 
            {
                let e = ["Неправильная схема управляющей команды получена схема: ", c, "сервером принимается только ", crate::ws_client_service::CLIENT_SCHEME].concat();
                fmt.write_str(&e)
            }
            WSError::ErrorDeserializeClientMessage(c) => 
            {
                let e = ["Ошибка десериализации входящего сообщения ", c].concat();
                fmt.write_str(&e)
            }
        }
    }
}

impl From<url::ParseError> for WSError
{
    fn from(value: url::ParseError) -> Self 
    {
        WSError::ErrorParsingClientCommand(value.to_string())
    }
}