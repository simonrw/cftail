use chrono::{DateTime, Utc};
use eyre::{Result, WrapErr};

pub(crate) fn parse_event_datetime(dt: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(dt)
        .wrap_err("parsing timestamp")?
        .with_timezone(&Utc))
}
