//! Only for tests.

use serde_json::Value;
use crate::error::Error;
use crate::request::client::Client;

/// Client for tests.
pub struct StubClient {
    expected_headers: Vec<(String, String)>,
    expected_params: Vec<(String, String)>,
    expected_payload: Value,
    response_body: Value,
}

impl StubClient {

    /// Create client.
    pub fn new(expected_headers: Vec<(String, String)>,
        expected_params: Vec<(String, String)>,
        expected_payload: Value, 
        response_body: Value) -> Self 
    {
        StubClient {
            expected_headers,
            expected_params,
            expected_payload,
            response_body,
        }
    }
}

impl Client for StubClient {

    fn make_json_request(&self, _url: &str, payload: Value, headers: &[(&str, &str)], params: &[(&str, &str)]) -> Result<Value, Error> {
        for (expected, actual) in headers.iter().zip(self.expected_headers.iter()) {
            assert_eq!(expected.0, actual.0, "headers keys");
            assert_eq!(expected.1, actual.1, "headers values");
        }

        for (expected, actual) in params.iter().zip(self.expected_params.iter()) {
            assert_eq!(expected.0, actual.0, "params keys");
            assert_eq!(expected.1, actual.1, "params values");
        }

        assert_eq!(payload, self.expected_payload);

        Ok(self.response_body.clone())
    }
}