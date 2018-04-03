use html;
use dom;
use css;
use style;
use layout;
use painter;
use window;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

extern crate gtk;
use gtk::WidgetExt;

extern crate app_units;
use app_units::Au;

extern crate reqwest;
use interface::reqwest::Url;

use std::fs;
use std::io::{BufWriter, Write};

extern crate rand;
use self::rand::Rng;

// If ``url_str`` starts with ``http(s)://``, downloads the specified file:
//  Returns (downloaded file name, file path(URL without ``http(s)://domain/``)).
// If ``url_str`` starts with ``file://``, doesn't do anything special.
//  Just returns (local file name, local file path).
pub fn download(url_str: &str) -> (String, PathBuf) {
    let url = HTML_SRC_URL.with(|a| {
        let mut a = a.borrow_mut();
        if let Some(ref mut a) = *a {
            let mut url = Url::parse(a.as_str()).unwrap();
            url.set_path(url_str);
            return url;
        }
        *a = Some(url_str.to_string());
        Url::parse(url_str).unwrap()
    });

    if url.scheme().to_ascii_lowercase() == "file" {
        // file://
        (url.path().to_string(), Path::new(url.path()).to_path_buf())
    } else {
        // http(s)://

        println!("download {}", url.as_str());

        let mut content: Vec<u8> = vec![];
        reqwest::get(url.clone())
            .unwrap()
            .copy_to(&mut content)
            .unwrap();
        let path = Path::new(url.path());

        let tmpfile_name = format!(
            "cache/{}.{}",
            rand::thread_rng()
                .gen_ascii_chars()
                .take(8)
                .collect::<String>(),
            if let Some(ext) = path.extension() {
                ext.to_str().unwrap()
            } else {
                "html"
            }
        );

        let mut f = BufWriter::new(fs::File::create(tmpfile_name.as_str()).unwrap());
        f.write_all(content.as_slice()).unwrap();

        (tmpfile_name, path.to_path_buf())
    }
}

use std::cell::RefCell;

thread_local!(
    pub static LAYOUT_SAVER: RefCell<(Au, Au, painter::DisplayList)> = {
        RefCell::new((Au(0), Au(0), vec![]))
    };

    pub static HTML_SRC_URL: RefCell<Option<String>> = {
        RefCell::new(None)
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
    let (html_src_cache_name, html_src_path) = download(html_src.as_str());

    println!("HTML:");
    let mut html_source = "".to_string();
    OpenOptions::new()
        .read(true)
        .open(html_src_cache_name)
        .unwrap()
        .read_to_string(&mut html_source)
        .ok()
        .expect("cannot read file");
    let html_tree = html::parse(html_source, html_src_path);
    print!("{}", html_tree);

    HTML_TREE.with(|h| {
        *h.borrow_mut() = Some(html_tree.clone());
    });

    println!("CSS:");
    let mut css_source = "".to_string();
    if let Some(stylesheet_path) = html_tree.find_stylesheet_path() {
        let (css_cache_name, _) = download(stylesheet_path.to_str().unwrap());
        OpenOptions::new()
            .read(true)
            .open(css_cache_name)
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
