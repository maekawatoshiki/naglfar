use html;
use dom;
use css;
use style;
use layout;
use painter;
use window;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

extern crate gtk;
use gtk::WidgetExt;

extern crate app_units;
use app_units::Au;

extern crate reqwest;

// pub fn aa() {
//     println!(
//         "body: {}",
//         reqwest::get("http://maekawatoshiki.github.io/naglfar/example/test.html")
//             .unwrap()
//             .text()
//             .unwrap()
//     );
// }

use std::cell::RefCell;

thread_local!(
    pub static LAYOUT_SAVER: RefCell<(Au, Au, painter::DisplayList)> = {
        RefCell::new((Au(0), Au(0), vec![]))
    };
    
    pub static HTML_TREE: RefCell<Option<dom::Node>> = {
        RefCell::new(None)
    };
    pub static STYLESHEET: RefCell<Option<css::Stylesheet>> = {
        RefCell::new(None)
    };
);

static mut SRC_UPDATED: bool = false;

pub fn update_html_tree_and_stylesheet(html_src: String) {
    let html_src_path = Path::new(html_src.as_str());

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

    HTML_TREE.with(|h| {
        *h.borrow_mut() = Some(html_tree.clone());
    });

    println!("CSS:");
    let mut css_source = "".to_string();
    if let Some(stylesheet_path) = html_tree.find_stylesheet_path() {
        OpenOptions::new()
            .read(true)
            .open(stylesheet_path.to_str().unwrap())
            .unwrap()
            .read_to_string(&mut css_source)
            .ok()
            .expect("cannot read file");
    } else {
        println!("*** Not found any stylesheet but continue ***");
    }
    let stylesheet = css::parse(css_source);
    print!("{}", stylesheet);

    STYLESHEET.with(|s| *s.borrow_mut() = Some(stylesheet));

    unsafe {
        SRC_UPDATED = true;
    }
}

pub fn run_with_url(html_src: String) {
    update_html_tree_and_stylesheet(html_src);

    window::render(move |widget| {
        let mut viewport: layout::Dimensions = ::std::default::Default::default();
        viewport.content.width = Au::from_f64_px(widget.get_allocated_width() as f64);
        viewport.content.height = Au::from_f64_px(widget.get_allocated_height() as f64);

        LAYOUT_SAVER.with(|x| {
            let (ref mut last_width, ref mut last_height, ref mut last_displays) = *x.borrow_mut();
            unsafe {
                if *last_width == viewport.content.width && *last_height == viewport.content.height
                    && !SRC_UPDATED
                {
                    last_displays.clone()
                } else {
                    SRC_UPDATED = false;
                    *last_width = viewport.content.width;
                    *last_height = viewport.content.height;

                    let html_tree = HTML_TREE.with(|h| (*h.borrow()).clone().unwrap());
                    let stylesheet = STYLESHEET.with(|s| (*s.borrow()).clone().unwrap());
                    let style_tree =
                        style::style_tree(&html_tree, &stylesheet, &style::PropertyMap::new());
                    let layout_tree = layout::layout_tree(&style_tree, viewport);
                    print!("LAYOUT:\n{}", layout_tree);

                    let display_command = painter::build_display_list(&layout_tree);
                    println!("DISPLAY:\n{:?}", display_command);

                    *last_displays = display_command.clone();

                    display_command
                }
            }
        })
    });
}
