// Copyright (c) 2024 Jake Walker
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::error::Error;

use models::{ProcessOptions, ServerInfo};
use reqwest::{
    header::{self, HeaderMap},
    Client as HttpClient,
};
use serde_json::{json, Value};

pub mod models;

/// An instance of a client for downloading things from cobalt
pub struct Client {
    /// HTTP client which requests to the cobalt server are made with
    client: HttpClient,
    /// The cobalt instance URL
    instance_url: String,
}

impl Client {
    /// Create a new cobalt client.
    ///
    /// # Examples
    ///
    /// ```
    /// use codl::Client;
    ///
    /// let my_client = Client::new(
    ///     "http://127.0.0.1:9000".to_string(),
    ///     "00000000-0000-0000-0000-000000000000".to_string()).unwrap();
    /// ```
    pub fn new(instance_url: String, auth_token: String) -> Result<Self, Box<dyn Error>> {
        let mut default_headers = HeaderMap::new();

        default_headers.insert(
            header::AUTHORIZATION,
            format!("Api-Key {}", auth_token).parse()?,
        );
        default_headers.insert(header::ACCEPT, "application/json".parse()?);

        Ok(Client {
            client: HttpClient::builder()
                .default_headers(default_headers)
                .build()?,
            instance_url,
        })
    }

    /// Get basic information about the cobalt instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use codl::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let my_client = Client::new(
    ///         "http://127.0.0.1:9000".to_string(),
    ///         "00000000-0000-0000-0000-000000000000".to_string()).unwrap();
    ///     let info = my_client.info().await;
    /// }
    /// ```
    pub async fn info(&self) -> Result<ServerInfo, Box<dyn Error>> {
        let res = self
            .client
            .get(&self.instance_url)
            .send()
            .await?
            .error_for_status()?;

        Ok(res.json::<ServerInfo>().await?)
    }

    /// Process media on the cobalt instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use codl::Client;
    /// use codl::models::ProcessOptions;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let my_client = Client::new(
    ///         "http://127.0.0.1:9000".to_string(),
    ///         "00000000-0000-0000-0000-000000000000".to_string()).unwrap();
    ///     let res = my_client.process(
    ///         "https://twitter.com/i/status/1825427547108053062",
    ///         ProcessOptions::default()).await;
    /// }
    /// ```
    pub async fn process(
        &self,
        url: &str,
        options: ProcessOptions,
    ) -> Result<String, Box<dyn Error>> {
        let mut body = serde_json::to_value(options)?;

        if let Value::Object(map) = &mut body {
            map.insert("url".to_string(), json!(url));
        }

        println!("{}", body.to_string());

        let res = self
            .client
            .post(&self.instance_url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(res.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTANCE_URL: &str = "http://127.0.0.1:9000";
    const AUTH_TOKEN: &str = "00000000-0000-0000-0000-000000000000";

    const MEDIA_URL: &str = "https://twitter.com/i/status/1825427547108053062";

    fn create_test_client() -> Result<Client, Box<dyn Error>> {
        Client::new(INSTANCE_URL.to_string(), AUTH_TOKEN.to_string())
    }

    #[tokio::test]
    async fn test_integration_info_is_ok() -> Result<(), Box<dyn Error>> {
        let client = create_test_client()?;
        client.info().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_process() -> Result<(), Box<dyn Error>> {
        let client = create_test_client()?;
        client.process(MEDIA_URL, ProcessOptions::default()).await?;
        Ok(())
    }
}
