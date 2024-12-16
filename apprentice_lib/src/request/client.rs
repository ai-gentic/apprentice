use serde_json::Value;
use crate::error::Error;
use super::reqwest::ReqwestClient;

/// Request client.
pub trait Client {
    /// Send request and receive response.
    fn make_json_request(&self, url: &str, payload: Value, headers: &[(&str, &str)], params: &[(&str, &str)]) -> Result<Value, Error>;
}

/// Create reqwest client.
pub fn get_reqwest_client() -> Result<Box<dyn Client>, Error> {
    Ok(Box::new(ReqwestClient::new()))
}