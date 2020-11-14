#[derive(Debug)]
pub(crate) enum Error {
    CredentialTimeout,
    Http(rusoto_core::request::BufferedHttpResponse),
    Other(Box<dyn std::error::Error>),
}
