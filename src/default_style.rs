use css::*;

use std::collections::HashSet;
use std::cell::RefCell;

pub fn default_rules() -> Vec<Rule> {
    DEFAULT_RULES.with(|default_rules| default_rules.borrow().clone())
}

thread_local!(
    pub static DEFAULT_RULES: RefCell<Vec<Rule>> = {
        let mut rules = vec![];
        rule_universal(&mut rules);
        rule_html(&mut rules);
        rule_body(&mut rules);
        rule_span(&mut rules);
        rule_h1(&mut rules);
        rule_h2(&mut rules);
        rule_h3(&mut rules);
        rule_a(&mut rules);
        rule_img(&mut rules);
        RefCell::new(rules)
    }
);

macro_rules! tag_name { ($name:expr) => {
    Selector::Simple(SimpleSelector {
        tag_name: Some($name.to_string()), id: None, class: HashSet::new() })
}}

macro_rules! decl { ($name:expr, $( $val:expr ),*) => {
    Declaration {
        name: $name.to_string(),
        values: vec![$($val)*],
    }
}}

macro_rules! keyword { ($str:expr) => { Value::Keyword($str.to_string()) }}
macro_rules! len_px  { ($val:expr) => { Value::Length($val, Unit::Px) }}
// macro_rules! num     { ($val:expr) => { Value::Num($val) }}
macro_rules! color   { ($clr:expr) => { Value::Color($clr) }}

fn rule_universal(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector {
                tag_name: None,
                id: None,
                class: HashSet::new(),
            }),
        ],
        declarations: vec![decl!("display", keyword!("block"))],
    });
}

fn rule_html(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("html")],
        declarations: vec![
            decl!("width", keyword!("auto")),
            decl!("padding", len_px!(0f64)),
            decl!("margin", len_px!(0f64)),
            decl!("background", color!(WHITE)),
        ],
    });
}

fn rule_body(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("body")],
        declarations: vec![
            decl!("padding", len_px!(0f64)),
            decl!("margin", len_px!(0f64)),
        ],
    });
}

fn rule_span(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("span")],
        declarations: vec![decl!("display", keyword!("inline"))],
    });
}

fn rule_h1(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("h1")],
        declarations: vec![
            decl!("font-size", len_px!(30f64)),
            decl!("font-weight", keyword!("bold")),
            decl!("padding", len_px!(10f64)),
        ],
    });
}

fn rule_h2(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("h2")],
        declarations: vec![
            decl!("font-size", len_px!(24f64)),
            decl!("font-weight", keyword!("bold")),
            decl!("padding", len_px!(10f64)),
        ],
    });
}

fn rule_h3(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("h3")],
        declarations: vec![
            decl!("font-size", len_px!(19f64)),
            decl!("font-weight", keyword!("bold")),
            decl!("padding", len_px!(10f64)),
        ],
    });
}

fn rule_a(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("a")],
        declarations: vec![
            decl!("display", keyword!("inline")),
            decl!(
                "color",
                color!(Color {
                    r: 0,
                    g: 0,
                    b: 0xee,
                    a: 0xff,
                })
            ),
            decl!("text-decoration", keyword!("underline")),
        ],
    });
}

fn rule_img(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![tag_name!("img")],
        declarations: vec![decl!("display", keyword!("inline"))],
    });
}
