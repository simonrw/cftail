use rusoto_cloudformation::DescribeStackEventsError;
use rusoto_core::{request::BufferedHttpResponse, RusotoError};
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::str::FromStr;

#[derive(Debug, PartialEq, Deserialize)]
pub(crate) struct ErrorResponse {
    #[serde(rename = "Error")]
    error: ErrorDetail,
    #[serde(rename = "RequestId")]
    request_id: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub(crate) struct ErrorDetail {
    #[serde(rename = "Type")]
    type_: String,
    #[serde(rename = "Code")]
    code: String,
    #[serde(rename = "Message")]
    message: String,
}

impl FromStr for ErrorResponse {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_xml_rs::from_str(s).map_err(|e| format!("{}", e))
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    CredentialTimeout,
    Http(BufferedHttpResponse),
    Rusoto(RusotoError<DescribeStackEventsError>),
    Other(Box<dyn std::error::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CredentialTimeout => f.write_str("your credentials have timed out"),
            Error::Http(_) => f.write_str("http error"),
            Error::Rusoto(_) => f.write_str("rusoto error"),
            Error::Other(e) => f.write_fmt(format_args!("other error: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rate_limiting_error() {
        let message =
            "<ErrorResponse xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">
            <Error>
                <Type>Sender</Type>
                <Code>Throttling</Code>
                <Message>Rate exceeded</Message>
            </Error>
            <RequestId>989a8c1f-735f-443e-8a77-ac87abf2b027</RequestId>
        </ErrorResponse>";
        let expected = ErrorResponse {
            error: ErrorDetail {
                message: "Rate exceeded".to_string(),
                code: "Throttling".to_string(),
                type_: "Sender".to_string(),
            },
            request_id: "989a8c1f-735f-443e-8a77-ac87abf2b027".to_string(),
        };
        let error = ErrorResponse::from_str(message).unwrap();
        assert_eq!(error, expected);
    }
}
