use reqwest::{header::HeaderMap, Response};
use std::collections::HashMap;
use anyhow::{Result, Context};

#[derive(Debug, Clone)]
pub struct DownloadClient {
    c: reqwest::Client,
}

impl DownloadClient {
    pub fn new() -> Self {
        Self {
            c: reqwest::Client::new(),
        }
    }

    pub async fn get(&self, url: &str, headers: HeaderMap) -> Result<Response> {
        Ok(self
            .c
            .get(url)
            .headers(headers)
            .send()
            .await
            .with_context(|| format!("Fail to get response from url: {url}"))?)
    }

    pub async fn post(
        &self,
        url: &str,
        headers: HeaderMap,
        body: HashMap<String, &String>,
    ) -> Result<Response> {
        Ok(self
            .c
            .post(url)
            .headers(headers)
            .form(&body)
            .send()
            .await
            .with_context(|| format!("Fail to post request to url: {url}"))?)
    }
}
