use std::future::Future;
use std::time::Duration;
use tokio::time::delay_for;

pub(crate) async fn backoff<F, G, T, E>(count: u8, mut f: F) -> Result<T, E>
where
    F: FnMut() -> G,
    G: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut i = 0u8;
    loop {
        let sleep_time = (i as u64) + 1;
        let res = f().await;
        match res {
            Ok(r) => return Ok(r),
            Err(e) if i >= count => return Err(e),
            Err(e) => tracing::trace!(err = %e, "error"),
        }
        tracing::trace!(sleep_time, "backing off");
        delay_for(Duration::from_secs(sleep_time)).await;
        i += 1;
    }
}
