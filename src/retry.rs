use std::future::Future;

use logger::warn;

///Повторное выполнение функции  
/// __attempts__ количество повторов  (0 бесконечный повтор)
/// __delay__ задержка между повторами в секундах  
pub async fn retry<F, Fu, V, E>(mut attempts: u8, delay: u64, f: F) -> Result<V, E>
where F: Fn() -> Fu,
      Fu: Future<Output=Result<V, E>> 
{
    loop 
    {
        match f().await 
        {
            Ok(v) => return Ok(v),
            Err(e) if attempts == 1 => return Err(e),
            _ => 
            {
                if attempts != 0
                {
                    attempts -= 1;
                    warn!("Повторная попытка выполнения retry осталось {} попыток", attempts);
                }
                else 
                {
                    warn!("Повторная попытка выполнения retry осталось ∞ попыток");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            }
        };
    }
}