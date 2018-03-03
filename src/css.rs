use std::fmt;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
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
    // Pt,
    // Em,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

pub const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

pub const RED: Color = Color {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

pub const GREEN: Color = Color {
    r: 0,
    g: 255,
    b: 0,
    a: 255,
};

pub const BLUE: Color = Color {
    r: 0,
    g: 0,
    b: 255,
    a: 255,
};

impl Copy for Color {}

impl Value {
    pub fn to_px(&self) -> Option<f64> {
        match *self {
            Value::Length(f, Unit::Px) | Value::Num(f) => Some(f),
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
            Value::Keyword(ref color_name) => match color_name.to_uppercase().as_str() {
                "BLACK" => Some(BLACK),
                "WHITE" => Some(WHITE),
                "RED" => Some(RED),
                "GREEN" => Some(GREEN),
                "BLUE" => Some(BLUE),
                _ => None,
            },
            _ => None,
        }
    }
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        // ref: http://www.w3.org/TR/selectors/#specificity
        let Selector::Simple(ref simple) = *self;
        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();
        (a, b, c)
    }
}

pub fn parse(source: String) -> Stylesheet {
    let mut parser = Parser {
        pos: 0,
        input: source,
    };
    Stylesheet {
        rules: parser.parse_rules(),
    }
}

pub fn parse_attr_style(source: String) -> Vec<Declaration> {
    let mut decls = Vec::new();
    let mut parser = Parser {
        pos: 0,
        input: source,
    };
    loop {
        parser.consume_whitespace();
        if parser.eof() {
            break;
        }
        decls.push(parser.parse_declaration());
    }
    decls
}

pub fn parse_value(source: String) -> Value {
    let mut parser = Parser {
        pos: 0,
        input: source,
    };
    parser.parse_value()
}

fn valid_ident_char(c: char) -> bool {
    // TODO: other char codes?
    c.is_alphanumeric() || c == '-' || c == '_'
}

#[derive(Clone, Debug)]
struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule());
        }
        rules
    }

    fn parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations(),
        }
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => {
                    self.consume_char();
                    self.consume_whitespace();
                }
                '{' => break,
                c => panic!("Unexpected character {} in selector list", c),
            }
        }
        // Return selectors with highest specificity first, for use in matching.
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        selectors
    }

    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: Vec::new(),
        };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // universal selector
                    self.consume_char();
                }
                c if valid_ident_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break,
            }
        }
        selector
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        assert_eq!(self.consume_char(), '{');
        let mut declarations = Vec::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declarations.push(self.parse_declaration());
        }
        declarations
    }

    fn parse_declaration(&mut self) -> Declaration {
        let property_name = self.parse_identifier();
        self.consume_whitespace();
        assert_eq!(self.consume_char(), ':');
        self.consume_whitespace();
        let value = self.parse_value();
        self.consume_whitespace();
        assert_eq!(self.consume_char(), ';');

        Declaration {
            name: property_name,
            value: value,
        }
    }

    // Methods for parsing values:

    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'...'9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Value::Keyword(self.parse_identifier()),
        }
    }

    fn parse_length(&mut self) -> Value {
        let num = self.parse_float();
        if !self.eof() && self.next_char().is_alphabetic() {
            Value::Length(num, self.parse_unit())
        } else {
            Value::Num(num)
        }
    }

    fn parse_float(&mut self) -> f64 {
        let s = self.consume_while(|c| match c {
            '0'...'9' | '.' => true,
            _ => false,
        });
        s.parse().unwrap()
    }

    fn parse_unit(&mut self) -> Unit {
        match &*self.parse_identifier().to_ascii_lowercase() {
            "px" => Unit::Px,
            _ => panic!("unrecognized unit"),
        }
    }

    fn parse_color(&mut self) -> Value {
        assert_eq!(self.consume_char(), '#');
        Value::Color(Color {
            r: self.parse_hex_pair(),
            g: self.parse_hex_pair(),
            b: self.parse_hex_pair(),
            a: 255,
        })
    }

    fn parse_hex_pair(&mut self) -> u8 {
        let s = &self.input[self.pos..self.pos + 2];
        self.pos += 2;
        u8::from_str_radix(s, 16).unwrap()
    }

    fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_ident_char)
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }

    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        cur_char
    }

    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

impl fmt::Display for Stylesheet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rule in &self.rules {
            for (i, selector) in rule.selectors.iter().enumerate() {
                let &Selector::Simple(ref selector) = selector;
                if let Some(ref id) = selector.id {
                    try!(write!(f, "#{}", id));
                } else if let Some(ref tag_name) = selector.tag_name {
                    try!(write!(f, "{}", tag_name));
                    for class in &selector.class {
                        try!(write!(f, ".{}", class));
                    }
                } else {
                    // universal selector
                    try!(write!(f, "*"));
                }
                if i != rule.selectors.len() - 1 {
                    try!(write!(f, ", "));
                }
            }
            try!(writeln!(f, " {{"));
            for decl in &rule.declarations {
                try!(writeln!(
                    f,
                    "  {}: {};",
                    decl.name,
                    match decl.value {
                        Value::Keyword(ref kw) => kw.clone(),
                        Value::Length(ref f, Unit::Px) => format!("{}px", f),
                        Value::Num(ref f) => format!("{}", f),
                        Value::Color(ref color) => {
                            format!("rgba({}, {}, {}, {})", color.r, color.g, color.b, color.a)
                        }
                    }
                ));
            }
            try!(writeln!(f, "}}"));
        }
        Ok(())
    }
}

#[test]
fn test1() {
    let src = "div, h1, #id, .class { width: 100px; height: 50px; font-weight: bold; z-index: 2; color: #ffffff; background-color: #003300; }";
    let stylesheet = parse(src.to_string());
    assert_eq!(
        stylesheet,
        Stylesheet {
            rules: vec![
                Rule {
                    selectors: vec![
                        Selector::Simple(SimpleSelector {
                            tag_name: None,
                            id: Some("id".to_string()),
                            class: vec![],
                        }),
                        Selector::Simple(SimpleSelector {
                            tag_name: None,
                            id: None,
                            class: vec!["class".to_string()],
                        }),
                        Selector::Simple(SimpleSelector {
                            tag_name: Some("div".to_string()),
                            id: None,
                            class: vec![],
                        }),
                        Selector::Simple(SimpleSelector {
                            tag_name: Some("h1".to_string()),
                            id: None,
                            class: vec![],
                        }),
                    ],
                    declarations: vec![
                        Declaration {
                            name: "width".to_string(),
                            value: Value::Length(100.0, Unit::Px),
                        },
                        Declaration {
                            name: "height".to_string(),
                            value: Value::Length(50.0, Unit::Px),
                        },
                        Declaration {
                            name: "font-weight".to_string(),
                            value: Value::Keyword("bold".to_string()),
                        },
                        Declaration {
                            name: "z-index".to_string(),
                            value: Value::Num(2.0),
                        },
                        Declaration {
                            name: "color".to_string(),
                            value: Value::Color(Color {
                                r: 0xff,
                                g: 0xff,
                                b: 0xff,
                                a: 0xff,
                            }),
                        },
                        Declaration {
                            name: "background-color".to_string(),
                            value: Value::Color(Color {
                                r: 0x00,
                                g: 0x33,
                                b: 0x00,
                                a: 0xff,
                            }),
                        },
                    ],
                },
            ],
        }
    );
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
                value: Value::Keyword("black".to_string()),
            },
            Declaration {
                name: "background".to_string(),
                value: Value::Keyword("white".to_string()),
            },
        ]
    );
}
