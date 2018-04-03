extern crate naglfar;
use naglfar::interface;

extern crate clap;
use clap::{App, Arg};

const VERSION_STR: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut app = App::new("Naglfar")
        .version(VERSION_STR)
        .author("uint256_t")
        .about("Naglfar is a web browser implementation in Rust")
        .arg(
            Arg::with_name("URL")
                .help("Set URL (starts with http(s):// or file://)")
                .index(1),
        );
    let app_matches = app.clone().get_matches();

    if let Some(url) = app_matches.value_of("URL") {
        interface::run_with_url(url.to_string())
    } else {
        app.print_help().unwrap();
        println!();
    }
}
