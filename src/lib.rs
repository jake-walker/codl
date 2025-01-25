// Copyright (c) 2024 Jake Walker
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::models::DownloadResult;
use models::{ProcessOptions, ProcessResult, ServerInfo};
use reqwest::header::HeaderValue;
use reqwest::Response;
use reqwest::{
    header::{self, HeaderMap},
    Client as HttpClient,
};
use serde_json::{json, Value};
use thiserror::Error;

pub mod models;

#[derive(Error, Debug)]
pub enum CodlError {
    #[error("invalid api token")]
    BadApiToken,
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error("problem parsing response")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("bad response from cobalt instance")]
    BadResponseError,
    #[error("cobalt error {0}")]
    CobaltError(String),
}

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
    pub fn new(instance_url: String, auth_token: Option<String>) -> Result<Self, CodlError> {
        let mut default_headers = HeaderMap::new();

        if let Some(token) = auth_token {
            default_headers.insert(
                header::AUTHORIZATION,
                format!("Api-Key {}", token)
                    .parse()
                    .map_err(|_| CodlError::BadApiToken)?,
            );
        }
        default_headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

        Ok(Client {
            client: HttpClient::builder()
                .default_headers(default_headers)
                .build()?,
            instance_url,
        })
    }

    async fn check_for_error(&self, res: Response) -> Result<Response, CodlError> {
        if res.status().is_success() {
            return Ok(res);
        }

        // try parsing cobalt error
        if let Ok(res_json) = res.json::<Value>().await {
            if let Some(obj) = res_json.as_object() {
                if obj.get("status").and_then(|v| v.as_str()) == Some("error") {
                    if let Some(error_code) = obj
                        .get("error")
                        .and_then(|v| v.as_object())
                        .and_then(|v| v.get("code"))
                        .and_then(|v| v.as_str())
                    {
                        return Err(CodlError::CobaltError(error_code.to_string()));
                    }
                }
            }
        }

        Err(CodlError::BadResponseError)
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
    pub async fn info(&self) -> Result<ServerInfo, CodlError> {
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
    ) -> Result<ProcessResult, CodlError> {
        let mut body = serde_json::to_value(options)?;

        if let Value::Object(map) = &mut body {
            map.insert("url".to_string(), json!(url));
        }

        let res = self
            .check_for_error(
                self.client
                    .post(&self.instance_url)
                    .json(&body)
                    .send()
                    .await?,
            )
            .await?;

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
            _ => Err(CodlError::BadResponseError),
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
    pub async fn process(&self, url: &str) -> Result<ProcessResult, CodlError> {
        self.process_with_options(url, ProcessOptions::default())
            .await
    }

    /// Download media using the cobalt instance with manual options.
    ///
    /// Please note that for picker items, the first will be chosen. If this isn't what you need, you should `process()` then handle the result accordingly.
    pub async fn download_with_options(
        &self,
        url: &str,
        options: ProcessOptions,
    ) -> Result<DownloadResult, CodlError> {
        let res = self.process_with_options(url, options).await?;

        let (url, filename) = {
            match res {
                ProcessResult::TunnelRedirect(t) => (t.url, t.filename),
                ProcessResult::Picker(p) => {
                    if let Some(picker_item) = p.picker.first() {
                        (picker_item.url.clone(), p.audio_filename)
                    } else {
                        return Err(CodlError::BadResponseError);
                    }
                }
            }
        };

        let download_res = reqwest::get(url).await?.error_for_status()?;

        Ok(DownloadResult {
            data: download_res.bytes().await?,
            filename,
        })
    }

    /// Download media using the cobalt instance with default options.
    ///
    /// Please note that for picker items, the first will be chosen. If this isn't what you need, you should `process()` then handle the result accordingly.
    pub async fn download(&self, url: &str) -> Result<DownloadResult, CodlError> {
        self.download_with_options(url, ProcessOptions::default())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha256;
    use sha256::Sha256Digest;

    const INSTANCE_URL: &str = "http://127.0.0.1:9000";

    const MEDIA_URL: &str = "https://twitter.com/i/status/1825427547108053062";

    fn create_test_client() -> Result<Client, CodlError> {
        Client::new(INSTANCE_URL.to_string(), None)
    }

    #[tokio::test]
    async fn test_integration_info_is_ok() -> Result<(), CodlError> {
        let client = create_test_client()?;
        client.info().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_process() -> Result<(), CodlError> {
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
    async fn test_integration_download() -> Result<(), CodlError> {
        let client = create_test_client()?;
        let res = client.download(MEDIA_URL).await?;

        assert_eq!(
            res.data.digest(),
            "a81e67228dd410fe68e68b07aa114e747c49bc34738d3f2fe87f88a32d1c2f57"
        );

        Ok(())
    }
}
