use tungstenite::{connect, Message};
use url::Url;

pub fn start_client()
{
    std::thread::spawn(||
    {
        let (mut socket, response) =
        connect(Url::parse("ws://127.0.0.1:3010/").unwrap()).expect("Can't connect");
        println!("Connected to the server");
        println!("Response HTTP code: {}", response.status());
        println!("Response contains the following headers:");
        for (ref header, _value) in response.headers() 
        {
            println!("* {}", header);
        }
        socket.send(Message::Text("Hello WebSocket".into())).unwrap();
        loop 
        {
            let msg = socket.read().expect("Error reading message");
            println!("Received: {}", msg);
        }
    });
}

#[cfg(test)]
mod test
{
    use logger::debug;
    use crate::WebsocketMessage;
    #[test]
    fn test_client()
    {
        logger::StructLogger::initialize_logger();
        super::start_client();
        loop 
        {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}