use reqwest;

fn get(client:&reqwest::Client, url:&str, headers: Option<reqwest::header::HeaderMap>) -> Result<String> {
    // TODO - If there's a list then support continuationToken
    println!("{}", url);
    let mut req = client.get(url);
    if let Some(extra_headers) = headers {
        req = req.headers(extra_headers)
    }
    let mut resp = req.send()?;
    resp.error_for_status_ref()?;
    let mut resp_body = match resp.content_length() {
        Some(len) => String::with_capacity(len as usize),
        None => String::new()
    };
    resp.read_to_string(&mut resp_body)?;
    Ok(resp_body)
}
