use ahash::AHashMap;
use chrono::{DateTime, Local};
use lib::Result;

use lazy_static::lazy_static;
use reqwest::header::{HeaderMap, HeaderName};

use crate::config::CONFIG;

lazy_static! {
    static ref CLIENT: reqwest::Client = client();
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResponseResult {
    pub(crate) code: u32,
    pub(crate) message: String,
    pub(crate) timestamp: DateTime<Local>,
    pub(crate) data: serde_json::Value,
}

pub(self) fn client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
}

pub(crate) async fn get(
    tls: bool,
    host: &str,
    uri: &str,
    query: Option<AHashMap<String, serde_json::Value>>,
    header: Option<AHashMap<String, serde_json::Value>>,
) -> Result<ResponseResult> {
    let mut str = String::new();
    if query.is_some() {
        let query = query.unwrap();
        for (key, value) in query {
            str.push_str(&format!("{}={}&", key, value));
        }
        if str.len() > 0 {
            str.pop();
        }
    }
    let url = if tls {
        format!("https://{}{}?{}", host, uri, str)
    } else {
        format!("http://{}{}?{}", host, uri, str)
    };
    let mut header_map = HeaderMap::new();
    if header.is_some() {
        let header = header.unwrap();
        for (key, value) in header {
            header_map.insert(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                value.to_string().parse().unwrap(),
            );
        }
    }
    let client = CLIENT.clone();
    let resp = client.get(url).headers(header_map).send().await?;
    let resp = resp.json::<ResponseResult>().await?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use ahash::AHashMap;
    use serde_json::json;

    #[tokio::test]
    async fn test() {
        let mut query = AHashMap::new();
        query.insert("user_id".to_owned(), json!(1));
        let resp = super::get(true, "127.0.0.1:11130", "/which_node", Some(query), None).await;
        println!("{:?}", resp);
    }
}
