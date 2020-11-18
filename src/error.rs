use rusoto_cloudformation::DescribeStackEventsError;
use rusoto_core::RusotoError;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("rusoto error {0}")]
    Rusoto(#[from] RusotoError<DescribeStackEventsError>),
    #[error("error parsing --since argument")]
    ParseSince,
    #[error("no credentials found")]
    NoCredentials,
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("credentials expired")]
    CredentialsExpired,
    #[error("no stack found")]
    NoStack,
    #[error("other error {0}")]
    Other(String),
}
