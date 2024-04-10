use std::collections::HashMap;

use anyhow::{Context, Result};
use log::{error, warn};
use reqwest::header::CONTENT_TYPE;
use reqwest::{blocking::Response, header::HeaderValue};

use crate::parser::parse_request;

/// Execute the http request defined in input_file. Returns
pub fn execute(
    input_file: &String,
    kwarg_map: HashMap<String, String>,
    db_conn: Option<rusqlite::Connection>,
) -> Result<Response> {
    // Load raw file
    let json = std::fs::read_to_string(input_file)
        .with_context(|| format!("failed to read from file={}", input_file.as_str()))?;

    // Fill in any context + render template
    let mut tera = tera::Tera::default();

    let _ = tera
        .add_raw_template("request_json", &json)
        .with_context(|| format!("failed to add template={}", &json))?;
    let mut context = tera::Context::new();
    for (key, value) in std::env::vars() {
        context.insert(format!("env_{}", key), &value);
    }
    for (key, value) in kwarg_map.iter() {
        context.insert(format!("arg_{}", key), value);
    }
    let rendered_json = tera
        .render("request_json", &context)
        .with_context(|| "failed to render template")?;

    // Parse json request
    let req = parse_request(&rendered_json).with_context(|| {
        format!(
            "failed to parse request json\nrequest={}",
            &rendered_json.as_str()
        )
    })?;

    // Execute request specified in json file
    let resp = req.send().with_context(|| "failed to send the request")?;

    // If the response has associated cookies, save to db
    if let Some(conn) = db_conn {
        for c in resp.cookies() {
            println!("cookie: {} {}", c.name(), c.value());
            let domain = c.domain().unwrap_or("");
            let path = c.path().unwrap_or("");
            let expiry = match c.expires() {
                Some(e) => {
                    let d = e
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .with_context(|| "failed to get expiry time")?;
                    d.as_secs().to_string()
                }
                None => "null".to_string(),
            };

            match conn.execute(
                "INSERT INTO cookies (name, value, domain, path, secure, http_only, expiry) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                &[
                    &c.name(),
                    &c.value(),
                    domain,
                    path,
                    c.secure().to_string().as_str(),
                    c.http_only().to_string().as_str(),
                    expiry.as_str()
                ]
            ) {
                Ok(_) => (),
                Err(e) => error!("failed to insert cookie into db, error={e}")
            }
        }
    }

    Ok(resp)
}

pub fn format_output(
    resp: Response,
    full_response: bool,
    pretty_print: bool,
    output_file: Option<String>,
) -> Result<String> {
    let mut response_string = String::new();
    let headers = resp.headers().to_owned();
    let status = resp.status().to_owned();
    let raw_body = resp
        .text()
        .with_context(|| "unable to decode response body")?;

    if full_response {
        response_string.push_str(format!("status: {status}\n").as_str());
        for (k, v) in headers.iter() {
            match v.to_str() {
                Ok(hv) => response_string.push_str(format!("{}: {}\n", k.to_string(), hv).as_str()),
                Err(e) => error!("Unable to convert header={k} to string, error={e}"),
            }
        }
    }
    if pretty_print {
        let content_type = headers.get(CONTENT_TYPE);
        match pretty_print_str(&raw_body, content_type) {
            Ok(b) => response_string.push_str(b.as_str()),
            Err(e) => {
                warn!("unable to pretty-print response, error={e}");
                response_string.push_str(raw_body.as_str())
            }
        }
    } else {
        response_string.push_str(raw_body.as_str())
    }

    if let Some(of) = output_file {
        let _ = std::fs::write(&of, &response_string)
            .with_context(|| format!("failed to write response to={}", of))?;
    }
    Ok(response_string)
}

pub fn pretty_print_str(body: &String, content_type: Option<&HeaderValue>) -> Result<String> {
    let mime_type: mime::Mime = content_type.unwrap().to_str()?.parse()?;
    match (mime_type.type_(), mime_type.subtype()) {
        (mime::APPLICATION, mime::JSON) => {
            let js_val: serde_json::Value = serde_json::from_str(body.as_str())?;
            Ok(serde_json::to_string_pretty(&js_val)?)
        }
        (t, st) => {
            warn!("unable to parse mime_type: ({t}, {st})");
            Ok(body.to_string())
        }
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_pretty_print() {}
}
