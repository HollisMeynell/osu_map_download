use anyhow::{Context, Result};
use lazy_static::lazy_static;
use reqwest::{header::HeaderMap, Response};
use std::collections::HashMap;

lazy_static! {
    /// A simple and global client
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

/// A wrapper function for sending HTTP GET request with given headers
pub async fn get(url: &str, headers: HeaderMap) -> Result<Response> {
    Ok(CLIENT
        .get(url)
        .headers(headers)
        .send()
        .await
        .with_context(|| format!("Fail to get response from url: {url}"))?)
}

/// A wrapper function for sending HTTP POST request with given headers and form
pub async fn post(
    url: &str,
    headers: HeaderMap,
    form: &HashMap<String, &String>,
) -> Result<Response> {
    Ok(CLIENT
        .post(url)
        .headers(headers)
        .form(form)
        .send()
        .await
        .with_context(|| format!("Fail to post request to url: {url}"))?)
}
