use html;
use dom;
use css;
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

/// If ``url_str`` starts with ``http(s)://``, downloads the specified file:
///  Returns (downloaded file name, file path(URL without ``http(s)://domain/``)).
/// If ``url_str`` starts with ``file://``, doesn't do anything special.
///  Just returns (local file name, local file path).
pub fn download(url_str: &str) -> (String, PathBuf) {
    let url = HTML_SRC_URL.with(|html_src_url| {
        let mut html_src_url = html_src_url.borrow_mut();
        if let Ok(parsed) = Url::parse(url_str) {
            // If url_str is absolute URL(starts with scheme://)
            *html_src_url = Some(url_str.to_string());
            return parsed;
        } else if let Some(ref mut html_src_url) = *html_src_url {
            let mut url = Url::parse(html_src_url.as_str()).unwrap();
            url.set_path(url_str);
            return url;
        }
        *html_src_url = Some(url_str.to_string());
        Url::parse(url_str).unwrap()
    });

    if url.scheme().to_ascii_lowercase() == "file" {
        // file://
        (url.path().to_string(), Path::new(url.path()).to_path_buf())
    } else {
        // http(s)://

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

        println!("downloaded {}", url.as_str());

        let mut f = BufWriter::new(fs::File::create(tmpfile_name.as_str()).unwrap());
        f.write_all(content.as_slice()).unwrap();

        (tmpfile_name, path.to_path_buf())
    }
}

use std::cell::RefCell;
use std::rc::Rc;

thread_local!(
    static LAYOUT_SAVER: RefCell<(Au, Au, painter::DisplayList)> = { RefCell::new((Au(0), Au(0), vec![])) };
    static HTML_SRC_URL: RefCell<Option<String>> = { RefCell::new(None) };
    static HTML_TREE: Rc<RefCell<Option<dom::Node>>> = { Rc::new(RefCell::new(None)) };
    static STYLESHEET: Rc<RefCell<Option<css::Stylesheet>>> = { Rc::new(RefCell::new(None)) };
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
    } else if let Some(stylesheet_str) = html_tree.find_stylesheet_in_style_tag() {
        css_source = stylesheet_str;
    } else {
        println!("*** Not found any stylesheet but continue ***");
    }
    let stylesheet = css::parse(css_source);
    print!("{}", stylesheet);

    HTML_TREE.with(|h| {
        *h.borrow_mut() = Some(html_tree);
    });
    STYLESHEET.with(|s| *s.borrow_mut() = Some(stylesheet));

    layout::STYLES.with(|s| s.borrow_mut().clear());

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
            if *last_width == viewport.content.width && *last_height == viewport.content.height
                && unsafe { !SRC_UPDATED }
            {
                last_displays.clone()
            } else {
                unsafe {
                    SRC_UPDATED = false;
                }
                *last_width = viewport.content.width;
                *last_height = viewport.content.height;

                let html_tree = HTML_TREE.with(|h| (*h.borrow()).clone().unwrap());
                let stylesheet = STYLESHEET.with(|s| (*s.borrow()).clone().unwrap());
                let layout_tree = layout::layout_tree(html_tree, &stylesheet, viewport);
                // print!("LAYOUT:\n{}", layout_tree);

                let display_command = painter::build_display_list(&layout_tree);
                // println!("DISPLAY:\n{:?}", display_command);

                *last_displays = display_command.clone();

                display_command
            }
        })
    });

    if let Ok(dir) = fs::read_dir("./cache") {
        for entry in dir {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(filename) = path.file_name() {
                    if filename.to_str().unwrap() == "README.md" {
                        continue;
                    }
                }
                fs::remove_file(path).expect("Failed to remove a file");
            };
        }
    }
}
