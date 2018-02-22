extern crate naglfar;
use naglfar::html;
use naglfar::css;
use naglfar::style;
use naglfar::layout;
use naglfar::painter;
use naglfar::window;

extern crate clap;
use clap::{App, Arg};

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

extern crate app_units;
use app_units::Au;

const VERSION_STR: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let app = App::new("naglfar")
        .version(VERSION_STR)
        .author("uint256_t")
        .about("naglfar is a web browser implementation in Rust")
        .arg(Arg::with_name("FILE").help("Input file").index(1));
    let _app_matches = app.get_matches();

    let src_path = Path::new("example");

    println!("HTML:");
    let mut html_source = "".to_string();
    OpenOptions::new()
        .read(true)
        .open(src_path.join("test.html").to_str().unwrap())
        .unwrap()
        .read_to_string(&mut html_source)
        .ok()
        .expect("cannot read file");
    let html_tree = html::parse(html_source);
    print!("{}", html_tree);

    println!("CSS:");
    let mut css_source = "".to_string();
    if let Some(stylesheet_path) = html_tree.find_stylesheet_path() {
        OpenOptions::new()
            .read(true)
            .open(src_path.join(stylesheet_path).to_str().unwrap())
            .unwrap()
            .read_to_string(&mut css_source)
            .ok()
            .expect("cannot read file");
    } else {
        println!("*** Not found any stylesheet but continue ***");
    }
    let stylesheet = css::parse(css_source);
    print!("{}", stylesheet);

    let mut viewport: layout::Dimensions = ::std::default::Default::default();
    viewport.content.width = Au::from_px(640);
    viewport.content.height = Au::from_px(480);

    window::render(&viewport, move |ctx| {
        let style_tree = style::style_tree(&html_tree, &stylesheet, &style::PropertyMap::new());
        let layout_tree = layout::layout_tree(&style_tree, ctx, viewport);
        print!("LAYOUT:\n{}", layout_tree);

        let display_command = painter::build_display_list(&layout_tree);
        println!("DISPLAY:\n{:?}", display_command);
        display_command
    });
}
