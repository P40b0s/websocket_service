use std::{fmt::Display};

use serde::{Deserialize, Serialize};

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


// #[derive(PartialEq)]
// pub enum PayloadTypeEnum
// {
//     String,
//     Number,
//     Object,
//     Array,
//     Command,
//     Unknown,
//     Error
// }
// impl Display for PayloadTypeEnum
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result 
//     {
//         match self 
//         {
//             Self::String => f.write_str("string"),
//             Self::Number => f.write_str("number"),
//             Self::Object => f.write_str("object"),
//             Self::Array => f.write_str("array"),
//             Self::Command => f.write_str("command"),
//             Self::Unknown => f.write_str("unknown"),
//             Self::Error => f.write_str("error"),
//         }
//     }
// }

// impl From<&String> for PayloadTypeEnum
// {
//     fn from(value: &String) -> Self 
//     {
//         match value.as_str()
//         {
//             "string" => Self::String,
//             "number" => Self::Number,
//             "object" => Self::Object,
//             "array" => Self::Array,
//             "command" => Self::Command,
//             "unknown" => Self::Unknown,
//             "error" => Self::Error,
//             _ => Self::Unknown
//         }
//     }
// } 