# Клиент-сервер websocket с общим типом сообщений
Клиент или сервер необходимо запускать в рантайме tokio  
При использовании future json или flexbuffers необходимо создать enum с перечнем сообщений
```rust
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum TransportMessage
{
    Test1(TestPayload),
    Test2(String)
}
```
При использовании future binary
```rust
#[derive(bitcode::Encode, bitcode::Decode, Debug)]
pub enum TransportMessage
{
    Test1(TestPayload),
    Test2(String)
}
```
Для сообщений необходимо реализовать трейт Converter
```rust
impl Converter for TransportMessage{}
```
Вот пример его реализации для future json (для всех футур он уже реализован надо его только имплементировать на свой enum)
```rust
pub trait Converter where Self: serde::Serialize, for <'de> Self : serde::Deserialize<'de> + Sized + Debug + Send
{
    fn from_binary(slice: &[u8]) -> Result<Self>
    {
        let obj = serde_json::from_slice::<Self>(slice).with_context(|| format!("Данный объект отличается от того который вы хотите получить"))?;
        return Ok(obj);
    }
    fn to_binary(self) -> Vec<u8>
    {
        let mut bytes: Vec<u8> = Vec::new();
        let _ = serde_json::to_writer(&mut bytes, &self);
        bytes
    }
}
```
## Клиент
Необходимо создать структуру и имплементировать для нее `Client`  
```rust
pub struct Client1;
impl Client<TransportMessage> for Client1{}
```
и можно запускать своего клиента так 
```rust
Client1::start_client("ws://127.0.0.1:3010/", |msg: TransportMessage|
{
    //обработка поступившего сообщения
    
}).await;
```
или так:
```rust
Client1::start_client("ws://127.0.0.1:3010/", on_client_receive).await;
fn on_client_receive(msg: TransportMessage)
{
   //обработка поступившего сообщения
}
```
## Сервер
Необходимо создать структуру и имплементировать для нее `Server`  
```rust
pub struct WsServer;
impl Server<TransportMessage> for WsServer{}
```

и можно запускать свой сервер так:
```rust
//addr - адрес от которого пришло сообщение
WsServer::start_server("127.0.0.1:3010", |addr, msg|
{
    async move
    {
        //обработка сообщения
    }
}).await;
```
или так:
```rust
WsServer::start_server("127.0.0.1:3010", on_server_receive).await;
async fn on_server_receive(addr: SocketAddr, msg: TransportMessage)
{
    //обработка сообщения
}
```

