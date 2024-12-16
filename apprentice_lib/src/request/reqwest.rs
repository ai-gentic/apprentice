use reqwest::blocking::Client as BlockingClient;
use serde_json::Value;
use crate::error::Error;
use crate::request::client::Client;

pub struct ReqwestClient {
    client: BlockingClient,
}

impl ReqwestClient {

    pub fn new() -> Self {
        ReqwestClient {
            client: BlockingClient::new(),
        }
    }
}

impl Client for ReqwestClient {

    fn make_json_request(&self, url: &str, payload: Value, headers: &[(&str, &str)], params: &[(&str, &str)]) -> Result<Value, Error> {

        let mut request = self.client
            .post(url)
            .query(params)
            .json(&payload);

        for (k, v) in headers {
            request = request.header(*k, *v);
        }

        let response = request.send()?;

        let ret = response.json()?;
        Ok(ret)
    }
}