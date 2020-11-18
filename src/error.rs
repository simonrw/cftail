use eyre::WrapErr;
use rusoto_cloudformation::DescribeStackEventsError;
use rusoto_core::RusotoError;
use serde::Deserialize;
use std::str::FromStr;

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
    #[error("general aws error response")]
    ErrorResponse(ErrorResponse),
    #[error("other error {0}")]
    Other(String),
}

#[derive(Debug, PartialEq, Deserialize)]
pub(crate) struct ErrorResponse {
    #[serde(rename = "Error")]
    pub(crate) error: ErrorDetail,
    #[serde(rename = "RequestId")]
    pub(crate) request_id: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub(crate) struct ErrorDetail {
    #[serde(rename = "Type")]
    pub(crate) type_: String,
    #[serde(rename = "Code")]
    pub(crate) code: String,
    #[serde(rename = "Message")]
    pub(crate) message: String,
}

impl FromStr for ErrorResponse {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_xml_rs::from_str(s).wrap_err_with(|| format!("parsing xml from {}", s))
    }
}
