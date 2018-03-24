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

extern crate gtk;
use gtk::WidgetExt;

extern crate app_units;
use app_units::Au;

use std::cell::RefCell;

thread_local!(
    pub static LAYOUT_SAVER: RefCell<(Au, Au, painter::DisplayList)> = {
        RefCell::new((Au(0), Au(0), vec![]))
    };
);

const VERSION_STR: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let app = App::new("Naglfar")
        .version(VERSION_STR)
        .author("uint256_t")
        .about("Naglfar is a web browser implementation in Rust")
        .arg(Arg::with_name("FILE").help("Input file").index(1));
    let _app_matches = app.get_matches();

    run_with_url("./example/test.html".to_string());
}

fn run_with_url(html_src: String) {
    let html_src_path = Path::new(html_src.as_str());
    let src_path = html_src_path.parent().unwrap();

    println!("HTML:");
    let mut html_source = "".to_string();
    OpenOptions::new()
        .read(true)
        .open(html_src_path.to_str().unwrap())
        .unwrap()
        .read_to_string(&mut html_source)
        .ok()
        .expect("cannot read file");
    let html_tree = html::parse(html_source, html_src_path.to_path_buf());
    print!("{}", html_tree);

    println!("CSS:");
    let mut css_source = "".to_string();
    if let Some(stylesheet_path) = html_tree.find_stylesheet_path() {
        let css_src_path = src_path.join(stylesheet_path);
        OpenOptions::new()
            .read(true)
            .open(css_src_path.to_str().unwrap())
            .unwrap()
            .read_to_string(&mut css_source)
            .ok()
            .expect("cannot read file");
    } else {
        println!("*** Not found any stylesheet but continue ***");
    }
    let stylesheet = css::parse(css_source);
    print!("{}", stylesheet);

    window::render(move |widget| {
        let mut viewport: layout::Dimensions = ::std::default::Default::default();
        viewport.content.width = Au::from_f64_px(widget.get_allocated_width() as f64);
        viewport.content.height = Au::from_f64_px(widget.get_allocated_height() as f64);
        LAYOUT_SAVER.with(|x| {
            let (ref mut last_width, ref mut last_height, ref mut last_displays) = *x.borrow_mut();
            if *last_width == viewport.content.width && *last_height == viewport.content.height {
                last_displays.clone()
            } else {
                *last_width = viewport.content.width;
                *last_height = viewport.content.height;

                let style_tree =
                    style::style_tree(&html_tree, &stylesheet, &style::PropertyMap::new());
                let layout_tree = layout::layout_tree(&style_tree, viewport);
                print!("LAYOUT:\n{}", layout_tree);

                let display_command = painter::build_display_list(&layout_tree);
                println!("DISPLAY:\n{:?}", display_command);

                *last_displays = display_command.clone();

                display_command
            }
        })
    });
}
