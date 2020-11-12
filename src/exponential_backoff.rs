use std::future::Future;
use std::time::Duration;
use tokio::time::delay_for;

pub(crate) async fn backoff<F, G, T, E>(mut f: F) -> Result<T, E>
where
    F: FnMut() -> G,
    G: Future<Output = Result<T, E>>,
{
    let mut i = 0u8;
    loop {
        let sleep_time = ((i as u64) + 1) * 1000;
        let res = f().await;
        match res {
            Ok(r) => return Ok(r),
            Err(e) if i > 2 => return Err(e),
            _ => {}
        }
        tracing::trace!(sleep_time, "backing off");
        delay_for(Duration::from_millis(sleep_time)).await;
        i += 1;
    }
}
