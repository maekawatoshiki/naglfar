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
        .arg(Arg::with_name("FILE").help("Input file").index(1));
    let app_matches = app.get_matches();

    interface::run_with_url(if let Some(filename) = app_matches.value_of("FILE") {
        filename.to_string()
    } else {
        "./example/test.html".to_string()
    });
}
