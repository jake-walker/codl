// Copyright (c) 2024 Jake Walker
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::error::Error;
use models::{ProcessOptions, ProcessResult, ServerInfo};
use reqwest::{
    header::{self, HeaderMap},
    Client as HttpClient,
};
use serde_json::{json, Value};
use crate::models::DownloadResult;

pub mod models;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

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
    ///     Some("00000000-0000-0000-0000-000000000000".to_string())).unwrap();
    /// ```
    pub fn new(instance_url: String, auth_token: Option<String>) -> Result<Self, GenericError> {
        let mut default_headers = HeaderMap::new();

        if let Some(token) = auth_token {
            default_headers.insert(header::AUTHORIZATION, format!("Api-Key {}", token).parse()?);
        }
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
    ///         Some("00000000-0000-0000-0000-000000000000".to_string())).unwrap();
    ///     let info = my_client.info().await;
    /// }
    /// ```
    pub async fn info(&self) -> Result<ServerInfo, GenericError> {
        let res = self
            .client
            .get(&self.instance_url)
            .send()
            .await?
            .error_for_status()?;

        Ok(res.json::<ServerInfo>().await?)
    }

    /// Process media on the cobalt instance with manual options.
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
    ///         Some("00000000-0000-0000-0000-000000000000".to_string())).unwrap();
    ///     let res = my_client.process_with_options(
    ///         "https://twitter.com/i/status/1825427547108053062",
    ///         ProcessOptions::default()).await;
    /// }
    /// ```
    pub async fn process_with_options(
        &self,
        url: &str,
        options: ProcessOptions,
    ) -> Result<ProcessResult, GenericError> {
        let mut body = serde_json::to_value(options)?;

        if let Value::Object(map) = &mut body {
            map.insert("url".to_string(), json!(url));
        }

        let res = self
            .client
            .post(&self.instance_url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let res_json: Value = res.json().await?;

        match res_json
            .as_object()
            .and_then(|x| x.get("status"))
            .and_then(|x| x.as_str())
        {
            Some("tunnel") | Some("redirect") => Ok(ProcessResult::TunnelRedirect(
                serde_json::from_value(res_json)?,
            )),
            Some("picker") => Ok(ProcessResult::Picker(serde_json::from_value(res_json)?)),
            _ => Err("bad response recieved".into()),
        }
    }

    /// Process media on the cobalt instance with default options.
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
    ///         Some("00000000-0000-0000-0000-000000000000".to_string())).unwrap();
    ///     let res = my_client.process("https://twitter.com/i/status/1825427547108053062").await;
    /// }
    /// ```
    pub async fn process(&self, url: &str) -> Result<ProcessResult, GenericError> {
        self.process_with_options(url, ProcessOptions::default()).await
    }

    pub async fn download_with_options(&self, url: &str, options: ProcessOptions) -> Result<DownloadResult, GenericError> {
        let res = self.process_with_options(url, options).await?;

        if let ProcessResult::Picker(_) = res {
            return Err("cannot download with picker response".into())
        } else if let ProcessResult::TunnelRedirect(data) = res {
            let download_res = reqwest::get(data.url).await?.error_for_status()?;

            return Ok(DownloadResult {
                data: download_res.bytes().await?,
                filename: data.filename,
            })
        }

        Err("invalid response received".into())
    }

    pub async fn download(&self, url: &str) -> Result<DownloadResult, GenericError> {
        self.download_with_options(url, ProcessOptions::default()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha256;
    use sha256::Sha256Digest;

    const INSTANCE_URL: &str = "http://127.0.0.1:9000";

    const MEDIA_URL: &str = "https://twitter.com/i/status/1825427547108053062";

    fn create_test_client() -> Result<Client, GenericError> {
        Client::new(INSTANCE_URL.to_string(), None)
    }

    #[tokio::test]
    async fn test_integration_info_is_ok() -> Result<(), GenericError> {
        let client = create_test_client()?;
        client.info().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_process() -> Result<(), GenericError> {
        let client = create_test_client()?;
        let res = client.process(MEDIA_URL).await?;

        if let ProcessResult::TunnelRedirect(res1) = res {
            assert_eq!(res1.filename, "twitter_1825427547108053062.mp4");
        } else {
            panic!("api response was not tunnel/redirect");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_integration_download() -> Result<(), GenericError> {
        let client = create_test_client()?;
        let res = client.download(MEDIA_URL).await?;

        assert_eq!(res.data.digest(), "a81e67228dd410fe68e68b07aa114e747c49bc34738d3f2fe87f88a32d1c2f57");

        Ok(())
    }
}
