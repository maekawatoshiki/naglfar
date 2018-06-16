use std::{fmt, collections::HashSet};

use html::remove_comments;

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
    Descendant(SimpleSelector, Box<Selector>),
    Child(SimpleSelector, Box<Selector>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f64, Unit),
    Num(f64),
    Color(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
    Pt,
    Percent,
    Em,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
    // Blink,
}

macro_rules! color { ($name:ident, $r:expr, $g:expr, $b:expr) => {
    pub const $name: Color = Color { r: $r, g: $g, b: $b, a: 0xff };
}}

color!(BLACK, 0x00, 0x00, 0x00);
color!(SILVER, 0xc0, 0xc0, 0xc0);
color!(GRAY, 0x80, 0x80, 0x80);
color!(WHITE, 0xff, 0xff, 0xff);
color!(RED, 0xff, 0x00, 0x00);
color!(MAROON, 0x80, 0x00, 0x00);
color!(PURPLE, 0x80, 0x00, 0x80);
color!(FUCHSIA, 0xff, 0x00, 0xff);
color!(GREEN, 0x00, 0x80, 0x00);
color!(LIME, 0x00, 0xff, 0x00);
color!(OLIVE, 0x80, 0x80, 0x00);
color!(YELLOW, 0xff, 0xff, 0x00);
color!(NAVY, 0x00, 0x00, 0x80);
color!(BLUE, 0x00, 0x00, 0xff);
color!(TEAL, 0x00, 0x80, 0x80);
color!(AQUA, 0x00, 0xff, 0xff);

impl Copy for Color {}

impl Value {
    pub fn to_px(&self) -> Option<f64> {
        match *self {
            Value::Length(f, Unit::Px) | Value::Num(f) => Some(f),
            Value::Length(f, Unit::Pt) => Some(pt2px(f)),
            _ => None,
        }
    }

    pub fn maybe_percent_to_px(&self, len: f64) -> Option<f64> {
        match *self {
            Value::Length(f, Unit::Px) | Value::Num(f) => Some(f),
            Value::Length(f, Unit::Pt) => Some(pt2px(f)),
            Value::Length(f, Unit::Percent) => Some(len * (f / 100.0)),
            _ => None,
        }
    }

    pub fn to_pt(&self) -> Option<f64> {
        match *self {
            Value::Length(f, Unit::Pt) | Value::Num(f) => Some(f),
            Value::Length(f, Unit::Px) => Some(px2pt(f)),
            _ => None,
        }
    }

    pub fn to_num(&self) -> f64 {
        match *self {
            Value::Num(f) => f,
            _ => 0.0,
        }
    }

    pub fn to_color(&self) -> Option<Color> {
        match *self {
            Value::Color(color) => Some(color),
            Value::Keyword(ref color_name) => match color_name.as_str() {
                "black" => Some(BLACK),
                "silver" => Some(SILVER),
                "gray" => Some(GRAY),
                "white" => Some(WHITE),
                "red" => Some(RED),
                "maroon" => Some(MAROON),
                "purple" => Some(PURPLE),
                "fuchsia" => Some(FUCHSIA),
                "green" => Some(GREEN),
                "lime" => Some(LIME),
                "olive" => Some(OLIVE),
                "yellow" => Some(YELLOW),
                "navy" => Some(NAVY),
                "blue" => Some(BLUE),
                "teal" => Some(TEAL),
                "aqua" => Some(AQUA),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn to_text_decoration(&self) -> Option<TextDecoration> {
        match *self {
            Value::Keyword(ref name) => match name.to_lowercase().as_str() {
                "none" => Some(TextDecoration::None),
                "underline" => Some(TextDecoration::Underline),
                "overline" => Some(TextDecoration::Overline),
                "line-through" => Some(TextDecoration::LineThrough),
                _ => None,
            },
            _ => None,
        }
    }
}

const DPI: f64 = 96.0;

// TODO: any other better way?
pub fn px2pt(f: f64) -> f64 {
    f / ((1.0 / 72.0) * DPI)
}

// TODO: any other better way?
pub fn pt2px(f: f64) -> f64 {
    f * ((1.0 / 72.0) * DPI)
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    // ref: http://www.w3.org/TR/selectors/#specificity
    pub fn specificity(&self) -> Specificity {
        fn specificity_simple(simple: &SimpleSelector) -> Specificity {
            let a = simple.id.iter().count();
            let b = simple.class.len();
            let c = simple.tag_name.iter().count();
            (a, b, c)
        }

        match *self {
            Selector::Simple(ref simple) => specificity_simple(simple),
            Selector::Descendant(ref a, ref b) => {
                let (a1, b1, c1) = specificity_simple(a);
                let (a2, b2, c2) = (*b).specificity();
                (a1 + a2, b1 + b2, c1 + c2)
            }
            Selector::Child(ref a, ref b) => {
                let (a1, b1, c1) = specificity_simple(a);
                let (a2, b2, c2) = (*b).specificity();
                (a1 + a2, b1 + b2, c1 + c2)
            }
        }
    }
}

pub fn parse(source: String) -> Stylesheet {
    Stylesheet {
        rules: Parser::new(source).parse_rules(),
    }
}

pub fn parse_attr_style(source: String) -> Vec<Declaration> {
    let mut decls = Vec::new();
    let mut parser = Parser::new(source);
    loop {
        parser.consume_whitespace().unwrap();
        if parser.eof() {
            break;
        }
        match parser.parse_declaration() {
            Ok(ok) => decls.push(ok),
            Err(_) => {}
        }
    }
    decls
}

pub fn parse_value(source: String) -> Value {
    match Parser::new(source).parse_value() {
        Ok(ok) => ok,
        Err(_) => Value::Num(0.0),
    }
}

fn valid_ident_char(c: char) -> bool {
    // TODO: other char codes?
    c.is_alphanumeric() || c == '-' || c == '_'
}

fn valid_ident_percent_char(c: char) -> bool {
    // TODO: other char codes?
    c.is_alphanumeric() || c == '%'
}

fn valid_alpha_percent_char(c: char) -> bool {
    // TODO: other char codes?
    c.is_alphanumeric() || c == '%'
}

fn valid_hex_char(c: char) -> bool {
    // TODO: other char codes?
    match c.to_ascii_lowercase() {
        'a' | 'b' | 'c' | 'd' | 'e' | 'f' => true,
        c if c.is_numeric() => true,
        _ => false,
    }
}

#[derive(Clone, Debug)]
struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn new(input: String) -> Parser {
        Parser {
            pos: 0,
            input: remove_comments(input.as_bytes(), "/*", "*/"),
        }
    }

    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = vec![];
        loop {
            self.consume_whitespace().unwrap();

            if self.eof() {
                break;
            }

            if self.next_char().unwrap() == '@' {
                // TODO: Implement correctly
                assert_eq!(self.consume_char().unwrap(), '@');
                self.parse_identifier().unwrap();
                self.consume_while(|c| c != '{').unwrap();
                self.consume_char().unwrap();
                loop {
                    self.consume_whitespace().unwrap();
                    if self.next_char().unwrap() == '}' {
                        break;
                    }
                    self.parse_rule().unwrap();
                }
                self.consume_char().unwrap();
            } else {
                if let Ok(ok) = self.parse_rule() {
                    rules.push(ok)
                }
            }
        }
        rules
    }

    fn parse_rule(&mut self) -> Result<Rule, ()> {
        Ok(Rule {
            selectors: self.parse_selectors()?,
            declarations: self.parse_declarations()?,
        })
    }

    fn parse_selectors(&mut self) -> Result<Vec<Selector>, ()> {
        let mut selectors = Vec::new();
        loop {
            if let Ok(ok) = self.parse_selector() {
                selectors.push(ok);
            }
            self.consume_whitespace()?;
            match self.next_char()? {
                ',' => {
                    self.consume_char()?;
                    self.consume_whitespace()?;
                }
                '{' => break,
                c => {
                    println!("Unexpected character {} in selector list", c);
                    // assert!(false);
                    self.consume_char()?;
                }
            }
        }
        // Return selectors with highest specificity first, for use in matching.
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        Ok(selectors)
    }

    fn parse_selector(&mut self) -> Result<Selector, ()> {
        let s1 = self.parse_simple_selector()?;
        self.consume_whitespace()?;
        match self.next_char()? {
            // Descendant
            c if c.is_alphanumeric() || c == '#' || c == '.' || c == ':' || c == '[' => {
                let s2 = self.parse_selector()?;
                return Ok(Selector::Descendant(s1, Box::new(s2)));
            }
            '>' => {
                assert_eq!(self.consume_char()?, '>');
                self.consume_whitespace()?;
                let s2 = self.parse_selector()?;
                return Ok(Selector::Child(s1, Box::new(s2)));
            }
            _ => {}
        }
        Ok(Selector::Simple(s1))
    }

    fn parse_simple_selector(&mut self) -> Result<SimpleSelector, ()> {
        let mut unsupported_feature = false;
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: HashSet::new(),
        };
        while !self.eof() {
            match self.next_char()? {
                '#' => {
                    self.consume_char()?;
                    selector.id = Some(self.parse_identifier()?);
                }
                '.' => {
                    self.consume_char()?;
                    selector.class.insert(self.parse_identifier()?);
                }
                '*' => {
                    // universal selector
                    self.consume_char()?;
                }
                ':' => {
                    self.parse_pseudo_class_or_element()?;
                }
                '[' => {
                    unsupported_feature = self.parse_attribute().is_err();
                }
                c if valid_ident_char(c) => {
                    selector.tag_name = Some(self.parse_identifier()?);
                }
                _ => break,
            }
        }
        if unsupported_feature {
            Err(())
        } else {
            Ok(selector)
        }
    }

    // TODO: Implement correctly
    fn parse_pseudo_class_or_element(&mut self) -> Result<(), ()> {
        assert_eq!(self.skip_char_if_any(':')?, true); // pseudo-class
        self.skip_char_if_any(':')?; //pseudo-element
        self.consume_whitespace()?;
        self.parse_identifier()?;
        self.consume_whitespace()?;
        if self.skip_char_if_any('(')? {
            self.consume_while(|c| c != ')')?;
            assert_eq!(self.consume_char()?, ')');
        }
        Ok(())
    }

    // TODO: Implement correctly
    fn parse_attribute(&mut self) -> Result<(), ()> {
        if self.skip_char_if_any('[')? {
            self.consume_while(|c| c != ']')?;
            assert_eq!(self.consume_char()?, ']');
        }
        // TODO: Just returns Err(()) to ignore this selector for now
        Err(())
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ()> {
        assert_eq!(self.consume_char()?, '{');
        let mut declarations = Vec::new();
        loop {
            self.consume_whitespace()?;
            if self.next_char()? == '}' {
                self.consume_char()?;
                break;
            }
            declarations.push(self.parse_declaration()?);
        }
        Ok(declarations)
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ()> {
        let property_name = self.parse_identifier()?;
        self.consume_whitespace()?;
        assert_eq!(self.consume_char()?, ':');
        self.consume_whitespace()?;
        let values = self.parse_values()?;
        self.consume_whitespace()?;

        Ok(Declaration {
            name: property_name,
            values: values,
        })
    }

    // Methods for parsing values:

    fn parse_values(&mut self) -> Result<Vec<Value>, ()> {
        let mut values = vec![];
        loop {
            self.consume_whitespace()?;
            if self.eof() {
                break;
            }
            if self.skip_char_if_any(';')? {
                break;
            }

            values.push(self.parse_value()?);

            self.consume_while(|c| c == ' ' || c == '\t')?;
            if self.skip_char_if_any('\n')? || self.skip_char_if_any('}')? {
                break;
            }

            self.skip_char_if_any(',')?;
        }
        Ok(values)
    }

    fn parse_value(&mut self) -> Result<Value, ()> {
        match self.next_char()? {
            '-' | '0'...'9' => self.parse_length(),
            '#' => self.parse_color(),
            '\"' | '\'' => self.parse_string(),
            _ => {
                let ident = self.parse_identifier()?;
                match ident.as_str() {
                    "rgb" => self.parse_rgb_color(),
                    "rgba" => self.parse_rgba_color(),
                    "url" => self.parse_url(),
                    _ => Ok(Value::Keyword(ident)),
                }
            }
        }
    }

    fn parse_length(&mut self) -> Result<Value, ()> {
        let num = self.parse_float()?;
        if !self.eof() && valid_alpha_percent_char(self.next_char()?) {
            Ok(Value::Length(num, self.parse_unit()?))
        } else {
            Ok(Value::Num(num))
        }
    }

    fn parse_float(&mut self) -> Result<f64, ()> {
        self.consume_while(|c| match c {
            '-' | '0'...'9' | '.' => true,
            _ => false,
        })?
            .parse()
            .or_else(|_| Err(()))
    }

    fn parse_string(&mut self) -> Result<Value, ()> {
        let quote = self.consume_char()?;
        self.consume_while(|c| c != quote)?;
        assert_eq!(self.consume_char()?, quote);
        // TODO: Implement correctly
        Ok(Value::Num(0.0))
    }

    fn parse_unit(&mut self) -> Result<Unit, ()> {
        match &*self.parse_identifier_percent()? {
            "px" => Ok(Unit::Px),
            "pt" => Ok(Unit::Pt),
            "%" => Ok(Unit::Percent),
            "em" => Ok(Unit::Em),
            _ => panic!("unrecognized unit"),
        }
    }

    fn parse_rgb_color(&mut self) -> Result<Value, ()> {
        assert_eq!(self.consume_char_ignore_whitescape()?, '(');
        let r = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ',');
        let g = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ',');
        let b = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ')');
        Ok(Value::Color(Color {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: 255,
        }))
    }

    fn parse_rgba_color(&mut self) -> Result<Value, ()> {
        assert_eq!(self.consume_char_ignore_whitescape()?, '(');
        let r = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ',');
        let g = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ',');
        let b = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ',');
        let a = self.parse_float()?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ')');
        Ok(Value::Color(Color {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: (255.0 * a) as u8,
        }))
    }

    fn parse_url(&mut self) -> Result<Value, ()> {
        // TODO: Implement correctly
        assert_eq!(self.consume_char_ignore_whitescape()?, '(');
        self.consume_while(|c| c != ')')?;
        assert_eq!(self.consume_char_ignore_whitescape()?, ')');
        Ok(Value::Num(0.0))
    }

    fn parse_color(&mut self) -> Result<Value, ()> {
        assert_eq!(self.consume_char()?, '#');
        let hex_str = self.parse_hex_num()?;
        let (r, g, b) = match hex_str.len() {
            3 => {
                let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap();
                let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap();
                let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap();
                (r * 16 + r, g * 16 + g, b * 16 + b)
            }
            6 => (
                u8::from_str_radix(&hex_str[0..2], 16).unwrap(),
                u8::from_str_radix(&hex_str[2..4], 16).unwrap(),
                u8::from_str_radix(&hex_str[4..6], 16).unwrap(),
            ),
            _ => panic!(),
        };
        Ok(Value::Color(Color {
            r: r,
            g: g,
            b: b,
            a: 255,
        }))
    }

    fn parse_hex_num(&mut self) -> Result<String, ()> {
        self.consume_while(valid_hex_char)
    }

    // fn parse_hex_pair(&mut self) -> Result<u8, ()> {
    //     let s = &self.input[self.pos..self.pos + 2];
    //     self.pos += 2;
    //     u8::from_str_radix(s, 16).unwrap()
    // }

    fn parse_identifier(&mut self) -> Result<String, ()> {
        Ok(self.consume_while(valid_ident_char)?.to_lowercase())
    }

    fn parse_identifier_percent(&mut self) -> Result<String, ()> {
        Ok(self.consume_while(valid_ident_percent_char)?.to_lowercase())
    }

    fn consume_char_ignore_whitescape(&mut self) -> Result<char, ()> {
        self.consume_whitespace()?;
        let c = self.consume_char()?;
        self.consume_whitespace()?;
        Ok(c)
    }

    fn consume_whitespace(&mut self) -> Result<(), ()> {
        self.consume_while(char::is_whitespace).and(Ok(()))
    }

    fn consume_while<F>(&mut self, f: F) -> Result<String, ()>
    where
        F: Fn(char) -> bool,
    {
        let mut s = "".to_string();
        while !self.eof() && f(self.next_char()?) {
            s.push(self.consume_char()?);
        }
        Ok(s)
    }
    fn consume_char(&mut self) -> Result<char, ()> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or(())?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }

    fn skip_char_if_any(&mut self, c: char) -> Result<bool, ()> {
        if !self.eof() && self.next_char()? == c {
            assert_eq!(self.consume_char()?, c);
            return Ok(true);
        }
        Ok(false)
    }

    fn next_char(&self) -> Result<char, ()> {
        self.input[self.pos..].chars().next().ok_or(())
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

impl fmt::Display for Stylesheet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rule in &self.rules {
            for (i, selector) in rule.selectors.iter().enumerate() {
                fn show(f: &mut fmt::Formatter, selector: &Selector) -> fmt::Result {
                    fn show_simple(
                        f: &mut fmt::Formatter,
                        selector: &SimpleSelector,
                    ) -> fmt::Result {
                        let mut universal = true;
                        if let Some(ref tag_name) = selector.tag_name {
                            universal = false;
                            try!(write!(f, "{}", tag_name));
                        }
                        if !selector.class.is_empty() {
                            universal = false;
                            for class in &selector.class {
                                try!(write!(f, ".{}", class))
                            }
                        }
                        if let Some(ref id) = selector.id {
                            universal = false;
                            try!(write!(f, "#{}", id));
                        }
                        if universal {
                            try!(write!(f, "*"))
                        }
                        Ok(())
                    }

                    match selector {
                        &Selector::Simple(ref selector) => show_simple(f, selector),
                        &Selector::Descendant(ref a, ref b) => {
                            try!(show_simple(f, &*a));
                            try!(write!(f, " "));
                            show(f, &*b)
                        }
                        &Selector::Child(ref a, ref b) => {
                            try!(show_simple(f, &*a));
                            try!(write!(f, " > "));
                            show(f, &*b)
                        }
                    }
                }
                try!(show(f, &selector));

                if i != rule.selectors.len() - 1 {
                    try!(write!(f, ", "));
                }
            }
            try!(writeln!(f, " {{"));
            for decl in &rule.declarations {
                try!(write!(f, "  {}:", decl.name,));
                for value in &decl.values {
                    try!(write!(
                        f,
                        " {}",
                        match value {
                            &Value::Keyword(ref kw) => kw.clone(),
                            &Value::Length(ref f, Unit::Px) => format!("{}px", f),
                            &Value::Length(ref f, Unit::Pt) => format!("{}pt", f),
                            &Value::Length(ref f, Unit::Percent) => format!("{}%", f),
                            &Value::Length(ref f, Unit::Em) => format!("{}em", f),
                            &Value::Num(ref f) => format!("{}", f),
                            &Value::Color(ref color) => {
                                format!("rgba({}, {}, {}, {})", color.r, color.g, color.b, color.a)
                            }
                        }
                    ))
                }
                try!(writeln!(f));
            }
            try!(writeln!(f, "}}"));
        }
        Ok(())
    }
}

#[test]
fn test1() {
    let src = "
        /* Comments... */
        div, h1, #id, .class, p > a, div p, * { 
            width: 70%; 
            height: 50px;
            font-weight: bold; 
            z-index: 2; 
            font-size: 10pt; 
            color: #ffffff; 
            background-color: #030; 
        }";
    let stylesheet = parse(src.to_string());
    let rules = vec![
        Rule {
            selectors: vec![
                Selector::Simple(SimpleSelector {
                    tag_name: None,
                    id: Some("id".to_string()),
                    class: HashSet::new(),
                }),
                Selector::Simple(SimpleSelector {
                    tag_name: None,
                    id: None,
                    class: {
                        let mut h = HashSet::new();
                        h.insert("class".to_string());
                        h
                    },
                }),
                Selector::Child(
                    SimpleSelector {
                        tag_name: Some("p".to_string()),
                        id: None,
                        class: HashSet::new(),
                    },
                    Box::new(Selector::Simple(SimpleSelector {
                        tag_name: Some("a".to_string()),
                        id: None,
                        class: HashSet::new(),
                    })),
                ),
                Selector::Descendant(
                    SimpleSelector {
                        tag_name: Some("div".to_string()),
                        id: None,
                        class: HashSet::new(),
                    },
                    Box::new(Selector::Simple(SimpleSelector {
                        tag_name: Some("p".to_string()),
                        id: None,
                        class: HashSet::new(),
                    })),
                ),
                Selector::Simple(SimpleSelector {
                    tag_name: Some("div".to_string()),
                    id: None,
                    class: HashSet::new(),
                }),
                Selector::Simple(SimpleSelector {
                    tag_name: Some("h1".to_string()),
                    id: None,
                    class: HashSet::new(),
                }),
                Selector::Simple(SimpleSelector {
                    tag_name: None,
                    id: None,
                    class: HashSet::new(),
                }),
            ],
            declarations: vec![
                Declaration {
                    name: "width".to_string(),
                    values: vec![Value::Length(70.0, Unit::Percent)],
                },
                Declaration {
                    name: "height".to_string(),
                    values: vec![Value::Length(50.0, Unit::Px)],
                },
                Declaration {
                    name: "font-weight".to_string(),
                    values: vec![Value::Keyword("bold".to_string())],
                },
                Declaration {
                    name: "z-index".to_string(),
                    values: vec![Value::Num(2.0)],
                },
                Declaration {
                    name: "font-size".to_string(),
                    values: vec![Value::Length(10.0, Unit::Pt)],
                },
                Declaration {
                    name: "color".to_string(),
                    values: vec![
                        Value::Color(Color {
                            r: 0xff,
                            g: 0xff,
                            b: 0xff,
                            a: 0xff,
                        }),
                    ],
                },
                Declaration {
                    name: "background-color".to_string(),
                    values: vec![
                        Value::Color(Color {
                            r: 0x00,
                            g: 0x33,
                            b: 0x00,
                            a: 0xff,
                        }),
                    ],
                },
            ],
        },
    ];
    assert_eq!(stylesheet, Stylesheet { rules: rules });
}

#[test]
fn test2() {
    let src = "color: black; background: white; ";
    let decls = parse_attr_style(src.to_string());

    assert_eq!(
        decls,
        vec![
            Declaration {
                name: "color".to_string(),
                values: vec![Value::Keyword("black".to_string())],
            },
            Declaration {
                name: "background".to_string(),
                values: vec![Value::Keyword("white".to_string())],
            },
        ]
    );
}

#[test]
fn test_rgb_rgba() {
    let src = "color: rgb(1, 2, 3); background: rgba(250, 1, 250, 0.3); ";
    let decls = parse_attr_style(src.to_string());

    assert_eq!(
        decls,
        vec![
            Declaration {
                name: "color".to_string(),
                values: vec![
                    Value::Color(Color {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 255,
                    }),
                ],
            },
            Declaration {
                name: "background".to_string(),
                values: vec![
                    Value::Color(Color {
                        r: 250,
                        g: 1,
                        b: 250,
                        a: (255.0 * 0.3) as u8,
                    }),
                ],
            },
        ]
    );
}
