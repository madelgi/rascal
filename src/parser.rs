use std::collections::HashMap;
use std::str::FromStr;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::{error, warn};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use serde_json::Value;

pub const AUTHORIZATION: &str = "authorization";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum HttpVersion {
    #[serde(rename = "HTTP/0.9")]
    V0_9,
    #[serde(rename = "HTTP/1.0")]
    V1_0,
    #[serde(rename = "HTTP/1.1")]
    V1_1,
    #[serde(rename = "HTTP/2.0")]
    V2,
    #[serde(rename = "HTTP/3.0")]
    V3,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum HttpMethod {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrUrl {
    String(String),
    Url(Url),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Url {
    protocol: Option<String>,
    host: String,
    port: Option<i16>,
    path: Option<String>,
    params: Option<HashMap<String, String>>,
    fragment: Option<String>,
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut url = String::new();
        if let Some(p) = &self.protocol {
            url.push_str(format!("{p}://").as_str());
        } else {
            url.push_str("https://");
        }
        url.push_str(&self.host.as_str());
        if let Some(p) = &self.port {
            url.push_str(format!(":{p}").as_str());
        }
        if let Some(p) = &self.path {
            url.push_str(p.as_str());
        }
        if let Some(params) = &self.params {
            let mut param_vec: Vec<String> = Vec::new();
            url.push_str("?");
            for (k, v) in params {
                param_vec.push(format!("{k}={v}"));
            }
            url.push_str(param_vec.join("&").as_str());
        }
        if let Some(f) = &self.fragment {
            url.push_str(format!("#{f}").as_str());
        }
        write!(f, "{}", url)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Auth {
    Basic { username: String, password: String },
    Bearer { token: String },
}

trait Authenticate {
    fn generate_auth_header(&self) -> String;
}

impl Authenticate for Auth {
    fn generate_auth_header(&self) -> String {
        match &self {
            Auth::Basic { username, password } => {
                format!(
                    "Basic {base64}",
                    base64 = BASE64_STANDARD
                        .encode(format!("{u}:{p}", u = username, p = password).as_bytes())
                )
            }
            Auth::Bearer { token } => {
                format!("Bearer {token}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    version: Option<HttpVersion>,
    method: HttpMethod,
    url: StringOrUrl,
    headers: Option<HashMap<String, String>>,
    body: Option<RequestBody>,
    auth: Option<Auth>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestBody {
    raw: Option<String>,
    filepath: Option<String>,
    json: Option<Value>,
}

impl ToString for RequestBody {
    fn to_string(&self) -> String {
        // Prioritize json
        if let Some(js) = &self.json {
            return js.to_string();
        } else if let Some(s) = &self.raw {
            return s.clone();
        } else if let Some(fp) = &self.filepath {
            match std::fs::read_to_string(fp) {
                Ok(content) => return content,
                Err(e) => {
                    error!("Unable to load {fp}, error={e}");
                    return "".to_string();
                }
            }
        }
        warn!("null request body");
        "".to_string()
    }
}

impl Request {
    pub fn send(&self) -> anyhow::Result<reqwest::blocking::Response> {
        let mut client_builder = reqwest::blocking::Client::builder();
        // Generate headers
        client_builder = client_builder.default_headers(self.build_headers()?);

        let client = client_builder.build()?;
        let response = match self.method {
            HttpMethod::GET => client.get(&self.build_url()).send(),
            HttpMethod::HEAD => client.head(&self.build_url()).send(),
            HttpMethod::POST => {
                let mut req = client.post(&self.build_url());
                if let Some(b) = &self.body {
                    req = req.body(b.to_string())
                }
                req.send()
            }
            HttpMethod::PUT => {
                let mut req = client.put(&self.build_url());
                if let Some(b) = &self.body {
                    req = req.body(b.to_string())
                }
                req.send()
            }
            HttpMethod::DELETE => todo!(),
            HttpMethod::CONNECT => todo!(),
            HttpMethod::OPTIONS => todo!(),
            HttpMethod::TRACE => todo!(),
            HttpMethod::PATCH => todo!(),
        };
        Ok(response?)
    }

    fn build_headers(&self) -> anyhow::Result<HeaderMap> {
        let mut header_map = HeaderMap::new();
        if let Some(h) = &self.headers {
            for (k, v) in h {
                let headername = HeaderName::from_str(k.to_lowercase().as_str())?;
                let headerval = HeaderValue::from_str(v.to_lowercase().as_str())?;
                header_map.insert(headername, headerval);
            }
        }

        // Special case for auth
        if let Some(a) = &self.auth {
            if header_map.contains_key(AUTHORIZATION) {
                warn!("Authorization header already exists, overwriting with auth block");
            }
            header_map.insert(AUTHORIZATION, a.generate_auth_header().parse().unwrap());
        }
        Ok(header_map)
    }

    fn build_url(&self) -> String {
        match &self.url {
            StringOrUrl::String(s) => s.clone(),
            StringOrUrl::Url(u) => u.to_string(),
        }
    }
}

pub fn parse_request(req_json: &String) -> Result<Request> {
    serde_json::from_str(req_json.as_str())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_http_version_serialize() {
        assert_eq!(
            serde_json::to_string(&HttpVersion::V0_9).unwrap(),
            "\"HTTP/0.9\""
        );
        assert_eq!(
            serde_json::to_string(&HttpVersion::V1_0).unwrap(),
            "\"HTTP/1.0\""
        );
        assert_eq!(
            serde_json::to_string(&HttpVersion::V1_1).unwrap(),
            "\"HTTP/1.1\""
        );
        assert_eq!(
            serde_json::to_string(&HttpVersion::V2).unwrap(),
            "\"HTTP/2.0\""
        );
        assert_eq!(
            serde_json::to_string(&HttpVersion::V3).unwrap(),
            "\"HTTP/3.0\""
        );
    }

    #[test]
    fn test_http_version_deserialize() {
        let v: HttpVersion = serde_json::from_str("\"HTTP/0.9\"").unwrap();
        assert_eq!(v, HttpVersion::V0_9);
        let v: HttpVersion = serde_json::from_str("\"HTTP/1.0\"").unwrap();
        assert_eq!(v, HttpVersion::V1_0);
        let v: HttpVersion = serde_json::from_str("\"HTTP/1.1\"").unwrap();
        assert_eq!(v, HttpVersion::V1_1);
        let v: HttpVersion = serde_json::from_str("\"HTTP/2.0\"").unwrap();
        assert_eq!(v, HttpVersion::V2);
        let v: HttpVersion = serde_json::from_str("\"HTTP/3.0\"").unwrap();
        assert_eq!(v, HttpVersion::V3);
    }

    #[test]
    fn test_http_method_serialize() {
        assert_eq!(serde_json::to_string(&HttpMethod::GET).unwrap(), "\"GET\"");
        assert_eq!(
            serde_json::to_string(&HttpMethod::HEAD).unwrap(),
            "\"HEAD\""
        );
        assert_eq!(
            serde_json::to_string(&HttpMethod::POST).unwrap(),
            "\"POST\""
        );
        assert_eq!(serde_json::to_string(&HttpMethod::PUT).unwrap(), "\"PUT\"");
        assert_eq!(
            serde_json::to_string(&HttpMethod::DELETE).unwrap(),
            "\"DELETE\""
        );
        assert_eq!(
            serde_json::to_string(&HttpMethod::CONNECT).unwrap(),
            "\"CONNECT\""
        );
        assert_eq!(
            serde_json::to_string(&HttpMethod::OPTIONS).unwrap(),
            "\"OPTIONS\""
        );
        assert_eq!(
            serde_json::to_string(&HttpMethod::TRACE).unwrap(),
            "\"TRACE\""
        );
        assert_eq!(
            serde_json::to_string(&HttpMethod::PATCH).unwrap(),
            "\"PATCH\""
        );
    }

    #[test]
    fn test_http_method_deserialize() {
        let m: HttpMethod = serde_json::from_str("\"GET\"").unwrap();
        assert_eq!(m, HttpMethod::GET);
        let m: HttpMethod = serde_json::from_str("\"HEAD\"").unwrap();
        assert_eq!(m, HttpMethod::HEAD);
        let m: HttpMethod = serde_json::from_str("\"POST\"").unwrap();
        assert_eq!(m, HttpMethod::POST);
        let m: HttpMethod = serde_json::from_str("\"PUT\"").unwrap();
        assert_eq!(m, HttpMethod::PUT);
        let m: HttpMethod = serde_json::from_str("\"DELETE\"").unwrap();
        assert_eq!(m, HttpMethod::DELETE);
        let m: HttpMethod = serde_json::from_str("\"CONNECT\"").unwrap();
        assert_eq!(m, HttpMethod::CONNECT);
        let m: HttpMethod = serde_json::from_str("\"OPTIONS\"").unwrap();
        assert_eq!(m, HttpMethod::OPTIONS);
        let m: HttpMethod = serde_json::from_str("\"TRACE\"").unwrap();
        assert_eq!(m, HttpMethod::TRACE);
        let m: HttpMethod = serde_json::from_str("\"PATCH\"").unwrap();
        assert_eq!(m, HttpMethod::PATCH);
    }

    #[test]
    fn test_auth_serialize() {
        let a = Auth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            "{\"type\":\"Basic\",\"username\":\"user\",\"password\":\"pass\"}"
        );
        let a = Auth::Bearer {
            token: "token".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            "{\"type\":\"Bearer\",\"token\":\"token\"}"
        );
    }

    #[test]
    fn test_auth_deserialize() {
        let a: Auth = serde_json::from_str(
            "{\"type\":\"Basic\",\"username\":\"user\",\"password\":\"pass\"}",
        )
        .unwrap();
        assert_eq!(
            a,
            Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string()
            }
        );
        let a: Auth = serde_json::from_str("{\"type\":\"Bearer\",\"token\":\"token\"}").unwrap();
        assert_eq!(
            a,
            Auth::Bearer {
                token: "token".to_string()
            }
        );
    }

    #[test]
    fn test_auth_generate_auth_header() {
        let a = Auth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        assert_eq!(a.generate_auth_header(), "Basic dXNlcjpwYXNz");
        let a = Auth::Bearer {
            token: "token".to_string(),
        };
        assert_eq!(a.generate_auth_header(), "Bearer token");
    }

    #[test]
    fn test_url_serialize() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        let u = Url {
            protocol: Some("https".to_string()),
            host: "example.com".to_string(),
            port: Some(8080),
            path: Some("/path".to_string()),
            params: Some(params),
            fragment: Some("fragment".to_string()),
        };
        assert_eq!(serde_json::to_string(&u).unwrap(), "{\"protocol\":\"https\",\"host\":\"example.com\",\"port\":8080,\"path\":\"/path\",\"params\":{\"key\":\"value\"},\"fragment\":\"fragment\"}");
    }

    #[test]
    fn test_url_deserialize() {
        let u: Url = serde_json::from_str("{\"protocol\":\"https\",\"host\":\"example.com\",\"port\":8080,\"path\":\"/path\",\"params\":{\"key\":\"value\"},\"fragment\":\"fragment\"}").unwrap();
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        assert_eq!(
            u,
            Url {
                protocol: Some("https".to_string()),
                host: "example.com".to_string(),
                port: Some(8080),
                path: Some("/path".to_string()),
                params: Some(params),
                fragment: Some("fragment".to_string())
            }
        );
    }

    #[test]
    fn test_url_to_string() {
        let u = Url {
            protocol: None,
            host: "example.com".to_string(),
            port: None,
            path: None,
            params: None,
            fragment: None,
        };
        assert_eq!(u.to_string(), "https://example.com");

        let mut params = HashMap::new();
        params.insert("key1".to_string(), "value1".to_string());
        params.insert("key2".to_string(), "value2".to_string());
        let u = Url {
            protocol: Some("https".to_string()),
            host: "example.com".to_string(),
            port: Some(8080),
            path: Some("/path".to_string()),
            params: Some(params),
            fragment: Some("fragment".to_string()),
        };
        assert_eq!(
            u.to_string(),
            "https://example.com:8080/path?key1=value1&key2=value2#fragment"
        );
    }

    #[test]
    fn test_request_serialize() {
        let mut headers = HashMap::new();
        headers.insert("headerkey".to_string(), "headervalue".to_string());
        let r = Request {
            version: Some(HttpVersion::V1_1),
            url: StringOrUrl::String("https://example.com".to_string()),
            method: HttpMethod::GET,
            headers: Some(headers),
            body: Some(RequestBody {
                raw: Some("raw".to_string()),
                filepath: Some("filepath".to_string()),
                json: Some(serde_json::json!({"key": "value"})),
            }),
            auth: Some(Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            }),
        };
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            serde_json::json!({
                "version": "HTTP/1.1",
                "method": "GET",
                "url": "https://example.com",
                "headers": {
                    "headerkey": "headervalue"
                },
                "body": {
                    "raw": "raw",
                    "filepath": "filepath",
                    "json": {
                        "key": "value"
                    }
                },
                "auth": {
                    "type": "Basic",
                    "username": "user",
                    "password": "pass"
                }
            })
            .to_string()
        );
    }

    #[test]
    fn test_request_deserialize() {
        let r: Request = serde_json::from_str(
            serde_json::json!({
                "version": "HTTP/1.1",
                "method": "GET",
                "url": "https://example.com",
                "headers": {
                    "headerkey": "headervalue"
                },
                "body": {
                    "raw": "raw",
                    "filepath": "filepath",
                    "json": {
                        "key": "value"
                    }
                },
                "auth": {
                    "type": "Basic",
                    "username": "user",
                    "password": "pass"
                }
            })
            .to_string()
            .as_str(),
        )
        .unwrap();
        let mut headers = HashMap::new();
        headers.insert("headerkey".to_string(), "headervalue".to_string());
        assert_eq!(
            r,
            Request {
                version: Some(HttpVersion::V1_1),
                url: StringOrUrl::String("https://example.com".to_string()),
                method: HttpMethod::GET,
                headers: Some(headers),
                body: Some(RequestBody {
                    raw: Some("raw".to_string()),
                    filepath: Some("filepath".to_string()),
                    json: Some(serde_json::json!({"key": "value"}))
                }),
                auth: Some(Auth::Basic {
                    username: "user".to_string(),
                    password: "pass".to_string()
                })
            }
        );
    }
}
