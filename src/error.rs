use rusoto_cloudformation::DescribeStackEventsError;
use rusoto_core::{request::BufferedHttpResponse, RusotoError};
use serde::Deserialize;
use std::str::FromStr;

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum AwsError {
    CredentialExpired,
    RateLimitExceeded,
    NoCredentials,
    NoStack,
}

impl FromStr for AwsError {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error_response = ErrorResponse::from_str(s)?;
        match error_response.error.code.as_str() {
            "Throttling" => Ok(Self::RateLimitExceeded),
            "ExpiredToken" => Ok(Self::CredentialExpired),
            "ValidationError" => Ok(Self::NoStack),
            _ => Err(format!(
                "unknown response error type: {}",
                error_response.error.code
            )),
        }
    }
}

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
    Http(BufferedHttpResponse),
    Rusoto(RusotoError<DescribeStackEventsError>),
    Aws(AwsError),
    Printing,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(_) => f.write_str("http error"),
            Error::Rusoto(_) => f.write_str("rusoto error"),
            Error::Aws(e) => f.write_fmt(format_args!("aws error: {:?}", e)),
            Error::Printing => f.write_str("printing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod aws_error {
        use super::*;

        #[test]
        fn parse_rate_limiting_error() {
            let message = rate_limiting_message();

            let expected = AwsError::RateLimitExceeded;

            assert_eq!(AwsError::from_str(message).unwrap(), expected);
        }

        #[test]
        fn parse_expired_token() {
            let message = expired_token_message();

            let expected = AwsError::CredentialExpired;

            assert_eq!(AwsError::from_str(message).unwrap(), expected);
        }

        #[test]
        fn parse_stack_does_not_exist() {
            let message = stack_does_not_exist();

            let expected = AwsError::NoStack;

            assert_eq!(AwsError::from_str(message).unwrap(), expected);
        }
    }

    mod error_response {
        use super::*;

        #[test]
        fn parse_rate_limiting_error() {
            let message = rate_limiting_message();
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

        #[test]
        fn parse_expired_token() {
            let message = expired_token_message();

            let expected = ErrorResponse {
                error: ErrorDetail {
                    message: "The security token included in the request is expired".to_string(),
                    code: "ExpiredToken".to_string(),
                    type_: "Sender".to_string(),
                },
                request_id: "4a281eb4-4572-4906-8c3c-e2b1551e2868".to_string(),
            };
            let error = ErrorResponse::from_str(message).unwrap();
            assert_eq!(error, expected);
        }
    }

    fn expired_token_message() -> &'static str {
        "<ErrorResponse xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">
            <Error>
                <Type>Sender</Type>
                <Code>ExpiredToken</Code>
                <Message>The security token included in the request is expired</Message>
            </Error>
            <RequestId>4a281eb4-4572-4906-8c3c-e2b1551e2868</RequestId>
            </ErrorResponse>"
    }

    fn rate_limiting_message() -> &'static str {
        "<ErrorResponse xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">
            <Error>
                <Type>Sender</Type>
                <Code>Throttling</Code>
                <Message>Rate exceeded</Message>
            </Error>
            <RequestId>989a8c1f-735f-443e-8a77-ac87abf2b027</RequestId>
        </ErrorResponse>"
    }

    fn stack_does_not_exist() -> &'static str {
        "<ErrorResponse xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">
           <Error>
               <Type>Sender</Type>
               <Code>ValidationError</Code>
               <Message>Stack [test-stack] does not exist</Message>
           </Error>
           <RequestId>7c6bbd3b-ff75-4fc3-a802-6459af5f3ca5</RequestId>
         </ErrorResponse>"
    }
}
