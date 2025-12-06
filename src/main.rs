extern crate cgi;
use log::info;
use log::LevelFilter;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use lazy_static::lazy_static;
lazy_static! {
    static ref IS_LOGGER_INIT: bool = init_my_logger();
}

// --- Logger ---
#[cfg(feature = "log")]
fn is_logger_init() -> bool {
    return *IS_LOGGER_INIT;
}
#[cfg(not(feature = "log"))]
fn is_logger_init() -> bool {
    return false;
}
fn init_my_logger() -> bool {
    let _ = simple_logging::log_to_file("git-lfs-rust.log", LevelFilter::Info);
    true
}

// --- Response helpers ---
fn json_response(status_code: u16, json: &str) -> cgi::Response {
    cgi::binary_response(status_code, "application/json", json.as_bytes().to_vec())
}

// --- Utility functions ---
fn str_ends_with(haystack: &str, needle: &str) -> bool {
    haystack.ends_with(needle)
}
fn str_before<'a>(haystack: &'a str, needle: &str) -> &'a str {
    match haystack.rfind(needle) {
        Some(pos) => &haystack[..pos],
        None => haystack,
    }
}
fn slash_process(s: &str) -> String {
    let mut s = s.trim_start_matches('/').to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}
fn parse_query(query: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}
fn get_server_url(env: &std::collections::HashMap<String, String>) -> String {
    // REQUEST_SCHEME (or infer from HTTPS), HTTP_HOST, SERVER_PORT, SCRIPT_NAME
    let scheme = if let Some(s) = env.get("REQUEST_SCHEME") {
        s.to_string()
    } else if let Some(https) = env.get("HTTPS") {
        if https == "on" || https == "1" { "https".to_string() } else { "http".to_string() }
    } else {
        "http".to_string()
    };
    let host = env.get("HTTP_HOST")
        .or_else(|| env.get("SERVER_NAME"))
        .cloned()
        .unwrap_or_else(|| "localhost".to_string());
    let port = env.get("SERVER_PORT").cloned().unwrap_or_else(|| "80".to_string());
    let script_name = env.get("SCRIPT_NAME").cloned().unwrap_or_default();

    let port_part = if (scheme == "http" && port == "80") || (scheme == "https" && port == "443") {
        String::new()
    } else {
        format!(":{}", port)
    };

    let mut base_path = script_name.trim_end_matches('/').to_string();
    if let Some(pos) = base_path.rfind('/') {
        base_path = base_path[..=pos].to_string();
    }
    if !base_path.ends_with('/') { base_path.push('/'); }
    format!("{}://{}{}{}", scheme, host, port_part, base_path)
}

// --- Main handler ---
cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
    if is_logger_init() {
        info!("Hello to logger!");
    }

    let uri = request.uri().to_string();
    let api = str_before(&uri, "?");
    let query = request.uri().query().unwrap_or("");
    let accept = request.headers().get("accept").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    // If you need server_url, you must reconstruct it from available request data or remove its usage.
    let server_url = "".to_string(); // Placeholder or reconstruct as needed
    let mut dir = String::new();

    // /locks/verify
    if str_ends_with(api, "/locks/verify") {
        dir = slash_process(str_before(api, "/locks/verify"));
        let body = locks_verify(&request);
        return json_response(200, &body);
    }
    // /objects/batch
    else if str_ends_with(api, "/objects/batch") {
        dir = slash_process(str_before(api, "/objects/batch"));
        let body = objects_batch(&request, &server_url, &dir);
        return json_response(200, &body);
    }
    // /upload
    else if str_ends_with(api, "/upload") {
        dir = slash_process(str_before(api, "/upload"));
        let params = parse_query(query);
        let resp = upload(&request, &dir, &params);
        return resp;
    }
    // /download
    else if str_ends_with(api, "/download") {
        dir = slash_process(str_before(api, "/download"));
        let params = parse_query(query);
        let resp = download(&dir, &params);
        return resp;
    }
    // 404
    else {
        return cgi::empty_response(404);
    }
} }

// --- Endpoint handlers ---

fn locks_verify(_request: &cgi::Request) -> String {
    // Always returns empty locks
    r#"{"ours":[],"theirs":[],"next_cursor":""}"#.to_string()
}

fn objects_batch(request: &cgi::Request, server_url: &str, dir: &str) -> String {
    let input = String::from_utf8_lossy(request.body()).to_string();
    let input_json: serde_json::Value = serde_json::from_str(&input).unwrap_or(serde_json::json!({}));
    let operation = input_json.get("operation").and_then(|v| v.as_str()).unwrap_or("");
    let objects = input_json.get("objects").and_then(|v| v.as_array()).cloned().unwrap_or(vec![]);
    let mut resp_objects = vec![];

    for mut o in objects {
        if let Some(oid) = o.get("oid").and_then(|v| v.as_str()) {
            let mut o_map = o.as_object().cloned().unwrap_or_default();
            o_map.insert("authenticated".to_string(), serde_json::json!(false));
            let path = format!("data/{}objects/{}", dir, oid);
            let exists = Path::new(&path).exists();
            let mut actions = o_map.get("actions").cloned().unwrap_or(serde_json::json!({}));
            if operation == "upload" && !exists {
                actions["upload"] = serde_json::json!({
                    "href": format!("{}/{}/upload?oid={}", server_url.trim_end_matches('/'), dir, oid),
                    "expires_in": 24 * 3600
                });
            }
            if operation == "download" && exists {
                actions["download"] = serde_json::json!({
                    "href": format!("{}/{}/download?oid={}", server_url.trim_end_matches('/'), dir, oid),
                    "expires_in": 24 * 3600
                });
            }
            o_map.insert("actions".to_string(), actions);
            resp_objects.push(serde_json::Value::Object(o_map));
        }
    }

    let resp = serde_json::json!({
        "transfer": "basic",
        "objects": resp_objects
    });
    resp.to_string()
}

fn upload(request: &cgi::Request, dir: &str, params: &HashMap<String, String>) -> cgi::Response {
    let oid = match params.get("oid") {
        Some(oid) if !oid.is_empty() => oid,
        _ => {
            return cgi::empty_response(404);
        }
    };
    let objects_dir = format!("data/{}objects", dir);
    let path = format!("{}/{}", objects_dir, oid);
    let path_obj = Path::new(&path);

    // Create directory if not exists
    if !Path::new(&objects_dir).exists() {
        if let Err(e) = fs::create_dir_all(&objects_dir) {
            return cgi::empty_response(500);
        }
    }
    // Write file if not exists
    if !path_obj.exists() {
        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                return cgi::empty_response(500);
            }
        };
        let body = request.body();
        if let Err(e) = file.write_all(body) {
            return cgi::empty_response(500);
        }
    }
    return cgi::empty_response(200);
}

fn download(dir: &str, params: &HashMap<String, String>) -> cgi::Response {
    let oid = match params.get("oid") {
        Some(oid) if !oid.is_empty() => oid,
        _ => {
            return cgi::empty_response(404);
        }
    };
    let path = format!("data/{}objects/{}", dir, oid);
    let path_obj = Path::new(&path);
    if path_obj.exists() {
        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => {
                return cgi::empty_response(500);
            }
        };
        let mut buf = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut buf).ok();
        return cgi::binary_response(200, "application/octet-stream", buf);
    } else {
        return cgi::empty_response(404);
    }
}