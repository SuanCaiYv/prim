use chrono::{DateTime, Local};
use lib::Result;

use lazy_static::lazy_static;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Version,
};

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
        .use_rustls_tls()
        .http2_prior_knowledge()
        .build()
        .unwrap()
}

pub(crate) async fn get(
    host: &str,
    uri: &str,
    query: &serde_json::Map<String, serde_json::Value>,
    headers: &serde_json::Map<String, serde_json::Value>,
) -> Result<ResponseResult> {
    let mut str = String::new();
    for (key, value) in query {
        if value.is_string() {
            str.push_str(&format!("{}={}&", key, value.as_str().unwrap()));
        } else {
            str.push_str(&format!("{}={}&", key, value.to_string()));
        }
    }
    if str.len() > 0 {
        str.pop();
    }
    let url = if str.len() == 0 {
        format!("https://{}{}", host, uri)
    } else {
        format!("https://{}{}?{}", host, uri, str)
    };
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        header_map.insert(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_str(value.as_str().unwrap()).unwrap(),
        );
    }
    let client = CLIENT.clone();
    let resp = client
        .get(url)
        .version(Version::HTTP_2)
        .headers(header_map)
        .send()
        .await?;
    let resp = resp.json::<ResponseResult>().await?;
    Ok(resp)
}

pub(crate) async fn put(
    host: &str,
    uri: &str,
    query: &serde_json::Map<String, serde_json::Value>,
    headers: &serde_json::Map<String, serde_json::Value>,
    body: Option<&serde_json::Value>,
) -> Result<ResponseResult> {
    let mut str = String::new();
    for (key, value) in query {
        if value.is_string() {
            str.push_str(&format!("{}={}&", key, value.as_str().unwrap()));
        } else {
            str.push_str(&format!("{}={}&", key, value.to_string()));
        }
    }
    if str.len() > 0 {
        str.pop();
    }
    let url = if str.len() == 0 {
        format!("https://{}{}", host, uri)
    } else {
        format!("https://{}{}?{}", host, uri, str)
    };
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        header_map.insert(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_str(value.as_str().unwrap()).unwrap(),
        );
    }
    let client = CLIENT.clone();
    let resp = match body {
        Some(body) => {
            client
                .put(url)
                .version(Version::HTTP_2)
                .headers(header_map)
                .json(body)
                .send()
                .await?
        }
        None => client.put(url).headers(header_map).send().await?,
    };
    let resp = resp.json::<ResponseResult>().await?;
    Ok(resp)
}

pub(crate) async fn post(
    host: &str,
    uri: &str,
    query: &serde_json::Map<String, serde_json::Value>,
    headers: &serde_json::Map<String, serde_json::Value>,
    body: Option<&serde_json::Value>,
) -> Result<ResponseResult> {
    let mut str = String::new();
    for (key, value) in query {
        if value.is_string() {
            str.push_str(&format!("{}={}&", key, value.as_str().unwrap()));
        } else {
            str.push_str(&format!("{}={}&", key, value.to_string()));
        }
    }
    if str.len() > 0 {
        str.pop();
    }
    let url = if str.len() == 0 {
        format!("https://{}{}", host, uri)
    } else {
        format!("https://{}{}?{}", host, uri, str)
    };
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        header_map.insert(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_str(value.as_str().unwrap()).unwrap(),
        );
    }
    let client = CLIENT.clone();
    let resp = client
        .post(url)
        // .version(Version::HTTP_2)
        .headers(header_map)
        .json(&body)
        .send()
        .await?;
    let resp = resp.json::<ResponseResult>().await?;
    Ok(resp)
}

pub(crate) async fn delete(
    host: &str,
    uri: &str,
    query: &serde_json::Map<String, serde_json::Value>,
    headers: &serde_json::Map<String, serde_json::Value>,
) -> Result<ResponseResult> {
    let mut str = String::new();
    for (key, value) in query {
        if value.is_string() {
            str.push_str(&format!("{}={}&", key, value.as_str().unwrap()));
        } else {
            str.push_str(&format!("{}={}&", key, value.to_string()));
        }
    }
    if str.len() > 0 {
        str.pop();
    }
    let url = if str.len() == 0 {
        format!("https://{}{}", host, uri)
    } else {
        format!("https://{}{}?{}", host, uri, str)
    };
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        header_map.insert(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_str(value.as_str().unwrap()).unwrap(),
        );
    }
    let client = CLIENT.clone();
    let resp = client
        .delete(url)
        // .version(Version::HTTP_2)
        .headers(header_map)
        .send()
        .await?;
    let resp = resp.json::<ResponseResult>().await?;
    Ok(resp)
}

// pub(crate) async fn upload(
//     host: &str,
//     uri: &str,
//     query: Option<AHashMap<String, serde_json::Value>>,
//     header: Option<AHashMap<String, serde_json::Value>>,
//     file: &str,
// ) -> Result<ResponseResult> {
//     let mut str = String::new();
//     if query.is_some() {
//         let query = query.unwrap();
//         for (key, value) in query {
//             str.push_str(&format!("{}={}&", key, value));
//         }
//         if str.len() > 0 {
//             str.pop();
//         }
//     }
//     let url = format!("https://{}{}?{}", host, uri, str);
//     let mut header_map = HeaderMap::new();
//     if header.is_some() {
//         let header = header.unwrap();
//         for (key, value) in header {
//             header_map.insert(
//                 HeaderName::from_bytes(key.as_bytes()).unwrap(),
//                 value.to_string().parse().unwrap(),
//             );
//         }
//     }
//     let client = CLIENT.clone();
//     let file = File::open(file).await?.into_std().await.;
//     let part = reqwest::multipart::Part::bytes().file_name("aaa.txt");
//     let resp = client
//         .post(url)
//         .headers(header_map)
//         .multipart(reqwest::multipart::Form::new().part("file", part))
//         .send()
//         .await?;
//     let resp = resp.json::<ResponseResult>().await?;
//     Ok(resp)
// }

#[cfg(test)]
mod tests {

    use serde_json::json;

    #[tokio::test]
    async fn test() {
        let mut query = serde_json::Map::new();
        query.insert("user_id".to_owned(), json!(1));
        let headers = json!(null);
        let empty_map = serde_json::Map::new();
        let resp = super::get(
            "127.0.0.1:11130",
            "/which_node",
            &query,
            headers.as_object().unwrap_or_else(|| &empty_map),
        )
        .await;
        println!("{:?}", resp);
    }
}
