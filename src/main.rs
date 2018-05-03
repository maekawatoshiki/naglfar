extern crate naglfar;
use naglfar::interface;

extern crate clap;
use clap::{App, Arg};

const VERSION_STR: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let app = App::new("Naglfar")
        .version(VERSION_STR)
        .author("uint256_t")
        .about("Naglfar is a web browser implementation in Rust")
        .arg(
            Arg::with_name("URL")
                .help("Set URL (starts with http(s):// or file://)")
                .index(1),
        );
    let app_matches = app.clone().get_matches();

    interface::run_with_url(if let Some(url) = app_matches.value_of("URL") {
        url.to_string()
    } else {
        let mut cur_dir = std::env::current_dir().unwrap();
        cur_dir.push("example");
        cur_dir.push("top.html");
        format!("file://{}", cur_dir.to_str().unwrap())
    });
}
