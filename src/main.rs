extern crate cgi;
use chrono::{Utc, Local, DateTime, Date};
use flexi_logger::{Logger, Criterion, Naming, Cleanup};
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
    // let _ = simple_logging::log_to_file("git-lfs-rust.log", LevelFilter::Info);
    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(flexi_logger::FileSpec::default().basename("git-lfs-rust").suffix("log"))
        .append()
        .rotate(
            Criterion::Size(10_000_000), // 10MB
            Naming::Numbers,
            Cleanup::KeepLogFiles(1),
        )
        .start()
        .unwrap();
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

// --- Repo dir extraction ---
fn extract_repo_dir(request: &cgi::Request, api: &str) -> String {
    if let Some(path_info) = request.headers().get("x-cgi-path-info").and_then(|v| v.to_str().ok()) {
        let path = path_info.trim_start_matches('/');
        let candidates = ["/objects", "/locks", "/upload", "/download"];
        for needle in candidates.iter() {
            if let Some(pos) = path.find(needle) {
                return path[..pos].to_string();
            }
        }
        return path.to_string();
    }
    let candidates = ["/objects", "/locks", "/upload", "/download"];
    let path = api.trim_start_matches('/');
    for needle in candidates.iter() {
        if let Some(pos) = path.find(needle) {
            return path[..pos].to_string();
        }
    }
    "".to_string()
}
fn parse_query(query: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

fn extract_script_name(request: &cgi::Request) -> String {
    if let Some(uri) = request.headers().get("x-cgi-request-uri").and_then(|v| v.to_str().ok()) {
        if let Some(pos) = uri.find(".cgi") {
            let end = pos + ".cgi".len();
            return uri[..end].to_string();
        }
    }
    // // fallback: envのSCRIPT_NAMEや空文字
    // request.env().get("SCRIPT_NAME").cloned().unwrap_or_default()
    return "git-lfs-rust.cgi".to_string();
}

fn get_server_url(request: &cgi::Request) -> String {
    let scheme = request.headers().get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    let host = request.headers().get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .or_else(|| request.headers().get("host").and_then(|v| v.to_str().ok()))
        .unwrap_or("localhost");

    let script_name = extract_script_name(&request);

    format!("{}://{}{}", scheme, host, script_name)
}

// --- Main handler ---
cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
    let uri = request.uri().to_string();
    let api = str_before(&uri, "?");
    let query = request.uri().query().unwrap_or("");
    let accept = request.headers().get("accept").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let server_url = get_server_url(&request);
    let mut dir = String::new();

    if is_logger_init() {
        let now = Local::now();
        info!("Access at {}", now.format("%Y-%m-%d %H:%M:%S"));
        info!("request uri: {}", request.uri());
        for (key, value) in request.headers().iter() {
            let value_str = value.to_str().unwrap_or("<invalid utf8>");
            info!("Header: {} = {}", key, value_str);
        }
    }

    // /version
    if str_ends_with(api, "/version") {
        let body = format!(
            r#"{{"version":"{}","name":"git-lfs-rust-cgi-server"}}"#,
            env!("CARGO_PKG_VERSION")
        );
        return json_response(200, &body);
    }
    // /test
    else if str_ends_with(api, "/test") {
        let body = r#"{"message":"This is a test endpoint","status":"ok"}"#;
        return json_response(200, body);
    }
    // /put_test
    else if str_ends_with(api, "/put_test") {
        let method = request.method().to_string();
        if method == "PUT" {
            let body = String::from_utf8_lossy(request.body()).to_string();
            let resp = format!(r#"{{"message":"PUT received!","data":"{}"}}"#, body);
            return json_response(200, &resp);
        } else {
            let resp = r#"{"message":"Not a PUT request."}"#;
            return json_response(200, resp);
        }
    }
    // /locks/verify
    else if str_ends_with(api, "/locks/verify") {
        dir = extract_repo_dir(&request, api);
        dir = slash_process(&dir);
        let body = locks_verify(&request);
        return json_response(200, &body);
    }
    // /objects/batch
    else if str_ends_with(api, "/objects/batch") {
        dir = extract_repo_dir(&request, api);
        dir = slash_process(&dir);
        let body = objects_batch(&request, &server_url, &dir);
        return json_response(200, &body);
    }
    // /upload
    else if str_ends_with(api, "/upload") {
        dir = extract_repo_dir(&request, api);
        dir = slash_process(&dir);
        let params = parse_query(query);
        let resp = upload(&request, &dir, &params);
        return resp;
    }
    // /download
    else if str_ends_with(api, "/download") {
        dir = extract_repo_dir(&request, api);
        dir = slash_process(&dir);
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
            let (d1, d2) = if oid.len() >= 4 {
                (&oid[0..2], &oid[2..4])
            } else {
                ("00", "00")
            };
            let path = format!("data/{}objects/{}/{}/{}", dir, d1, d2, oid);
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
    let (d1, d2) = if oid.len() >= 4 {
        (&oid[0..2], &oid[2..4])
    } else {
        ("00", "00")
    };
    let objects_dir = format!("data/{}objects/{}/{}", dir, d1, d2);
    let path = format!("{}/{}", objects_dir, oid);
    let path_obj = Path::new(&path);

    // Create directory if not exists
    if !Path::new(&objects_dir).exists() {
        if let Err(_e) = fs::create_dir_all(&objects_dir) {
            return cgi::empty_response(500);
        }
    }
    // Write file if not exists
    if !path_obj.exists() {
        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(_e) => {
                return cgi::empty_response(500);
            }
        };
        let body = request.body();
        if let Err(_e) = file.write_all(body) {
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
    let (d1, d2) = if oid.len() >= 4 {
        (&oid[0..2], &oid[2..4])
    } else {
        ("00", "00")
    };
    let path = format!("data/{}objects/{}/{}/{}", dir, d1, d2, oid);
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