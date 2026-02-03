use anyhow::Result;
use log::debug;
use serde::Serialize;
use std::io::Read;

pub fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent("wpt interop bot")
        .build()?)
}

pub fn get(
    client: &reqwest::blocking::Client,
    url: &str,
    headers: Option<reqwest::header::HeaderMap>,
) -> Result<String> {
    // TODO - If there's a list then support continuationToken
    debug!("GET {}", url);
    let mut req = client.get(url);
    if let Some(extra_headers) = headers {
        req = req.headers(extra_headers)
    }
    let mut resp = req.send()?;
    resp.error_for_status_ref()?;
    let mut resp_body = match resp.content_length() {
        Some(len) => String::with_capacity(len as usize),
        None => String::new(),
    };
    resp.read_to_string(&mut resp_body)?;
    Ok(resp_body)
}

pub fn post<T>(
    client: &reqwest::blocking::Client,
    url: &str,
    headers: Option<reqwest::header::HeaderMap>,
    body: Option<T>,
) -> Result<String>
where
    T: Serialize,
{
    // TODO - If there's a list then support continuationToken
    debug!("POST {}", url);
    let mut req = client.post(url);
    if let Some(extra_headers) = headers {
        req = req.headers(extra_headers)
    }
    if let Some(body) = body {
        let body_str = serde_json::to_string(&body)?;
        debug!("{}", body_str);
        req = req.body(body_str);
    }
    let mut resp = req.send()?;
    let mut resp_body = match resp.content_length() {
        Some(len) => String::with_capacity(len as usize),
        None => String::new(),
    };
    resp.read_to_string(&mut resp_body)?;
    resp.error_for_status_ref()?;
    Ok(resp_body)
}
