extern crate cgi;
use log::info;
use log::LevelFilter;

use lazy_static::lazy_static;
lazy_static! {
    static ref IS_LOGGER_INIT: bool = init_my_logger();
}

#[cfg(feature = "log")]
fn is_logger_init() -> bool {
     return *IS_LOGGER_INIT;
}

#[cfg(not(feature = "log"))]
fn is_logger_init() -> bool {
     return false;
}

fn init_my_logger() -> bool {
     // init_logger!("git-lfs-rust").unwrap();
     // let mut path_or_not = process_path::get_executable_path();
     // if let Some(mut path) = path_or_not {
     //      path.push("git-lfs-rust.log");
     //      let _ = simple_logging::log_to_file(path, LevelFilter::Info);
     // } else {
          let _ = simple_logging::log_to_file("git-lfs-rust.log", LevelFilter::Info);
     // }
     true
}

cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
     if is_logger_init() {
          info!("Hello to logger!");
     }
     cgi::text_response(200, "Hello World")
} }
