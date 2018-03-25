use dom;

use std::collections::HashMap;
use std::cell::RefCell;
use std::path::PathBuf;
use std::cmp::max;
use std::str::from_utf8;

thread_local!(
    pub static CUR_DIR: RefCell<PathBuf> = {
        RefCell::new(PathBuf::new())
    };
);

pub fn parse(source: String, file_path: PathBuf) -> dom::Node {
    CUR_DIR.with(|cur_dir| *cur_dir.borrow_mut() = file_path.parent().unwrap().to_path_buf());
    let mut nodes = Parser::new(source).parse_nodes();

    // If the document contains a root element, just return it. Otherwise, create one.
    if nodes.len() == 1 {
        nodes.swap_remove(0)
    } else {
        dom::Node::elem("html".to_string(), HashMap::new(), nodes)
    }
}

fn is_not_to_close_tag(tag_name: &str) -> bool {
    if tag_name == "br" || tag_name == "img" || tag_name == "hr" || tag_name == "meta"
        || tag_name == "input" || tag_name == "embed" || tag_name == "area"
        || tag_name == "base" || tag_name == "col" || tag_name == "keygen"
        || tag_name == "link" || tag_name == "param" || tag_name == "source"
    {
        true
    } else {
        false
    }
}

pub fn remove_comments(s: &[u8], opening: &str, closing: &str) -> String {
    let mut level = 0;
    let mut pos = 0;
    let mut ret = "".to_string();
    let len = s.len();
    let opening_len = opening.len();
    let closing_len = closing.len();

    if len as isize - max(opening_len, closing_len) as isize - 1 < 0 {
        return from_utf8(s).unwrap().to_string();
    }

    while pos < len {
        if pos < len - opening_len - 1 && s[pos..(pos + opening_len)] == *opening.as_bytes() {
            pos += opening_len;
            level += 1;
            continue;
        }
        if pos < len - closing_len - 1 && s[pos..(pos + closing_len)] == *closing.as_bytes() {
            pos += closing_len;
            if level <= 0 {
                panic!("not found corresponding \"/*\"")
            }
            level -= 1;
            continue;
        }
        if level == 0 {
            ret.push(s[pos] as char);
        }
        pos += 1;
    }

    if level != 0 {
        panic!("comments are not balanced")
    }

    ret
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn new(input: String) -> Parser {
        Parser {
            pos: 0,
            input: remove_comments(input.as_bytes(), "<!--", "-->"),
        }
    }

    fn parse_nodes(&mut self) -> Vec<dom::Node> {
        let mut nodes: Vec<dom::Node> = vec![];
        loop {
            // TODO: Is this correct?
            match nodes.last() {
                Some(last) if last.is_inline() && last.contains_text() => {}
                _ => self.consume_whitespace(),
            };
            if self.eof() || self.starts_with("</") {
                break;
            }
            nodes.push(self.parse_node());
        }
        nodes
    }

    fn parse_node(&mut self) -> dom::Node {
        match self.next_char() {
            '<' => self.parse_element(),
            _ => self.parse_text(),
        }
    }

    fn parse_element(&mut self) -> dom::Node {
        // Opening tag.
        assert_eq!(self.consume_char(), '<');
        let tag_name = self.parse_tag_name();
        let attrs = self.parse_attributes();
        assert_eq!(self.consume_char(), '>');

        if is_not_to_close_tag(tag_name.as_str()) {
            return dom::Node::elem(tag_name, attrs, vec![]);
        }

        // Contents.
        let children = self.parse_nodes();

        // Closing tag.
        assert_eq!(self.consume_char(), '<');
        assert_eq!(self.consume_char(), '/');
        assert_eq!(self.parse_tag_name(), tag_name);
        assert_eq!(self.consume_char(), '>');

        dom::Node::elem(tag_name, attrs, children)
    }

    fn parse_tag_name(&mut self) -> String {
        self.consume_while(|c| c.is_alphanumeric())
    }

    fn parse_attributes(&mut self) -> dom::AttrMap {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '>' {
                break;
            }
            let (name, value) = url_conv(self.parse_attr());
            attributes.insert(name, value);
        }
        attributes
    }

    fn parse_attr(&mut self) -> (String, String) {
        let name = self.parse_tag_name();
        assert_eq!(self.consume_char(), '=');
        let value = self.parse_attr_value();
        (name, value)
    }

    fn parse_attr_value(&mut self) -> String {
        let open_quote = self.consume_char();
        assert!(open_quote == '"' || open_quote == '\'');
        let value = self.consume_while(|c| c != open_quote);
        assert_eq!(self.consume_char(), open_quote);
        value
    }

    fn parse_text(&mut self) -> dom::Node {
        let mut last = '*'; // any char except space
        dom::Node::text(self.consume_while(|c| c != '<').chars().fold(
            "".to_string(),
            |mut s, c| {
                if !(last.is_whitespace() && c.is_whitespace()) {
                    s.push(if c.is_whitespace() { ' ' } else { c });
                }
                last = c;
                s
            },
        ))
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    fn consume_while<F>(&mut self, f: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && f(self.next_char()) {
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

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

fn url_conv(attr: (String, String)) -> (String, String) {
    if attr.0.to_lowercase().as_str() == "src" {
        (
            attr.0.clone(),
            CUR_DIR.with(|dir| dir.borrow().join(attr.1).to_str().unwrap().to_string()),
        )
    } else {
        (attr.0, attr.1)
    }
}

#[test]
fn test1() {
    use std::path::Path;
    let src = "<html><head></head><body><div id=\"x\">test</div><p>paragrapgh</p><span>aa</span>\n  space<img src='a.png'></body></html>";
    let dom_node = parse(src.to_string(), Path::new("./a/a.html").to_path_buf());
    assert_eq!(
        dom_node,
        dom::Node::elem(
            "html".to_string(),
            HashMap::new(),
            vec![
                dom::Node::elem("head".to_string(), HashMap::new(), vec![]),
                dom::Node::elem(
                    "body".to_string(),
                    HashMap::new(),
                    vec![
                        dom::Node::elem(
                            "div".to_string(),
                            {
                                let mut h = HashMap::new();
                                h.insert("id".to_string(), "x".to_string());
                                h
                            },
                            vec![dom::Node::text("test".to_string())],
                        ),
                        dom::Node::elem(
                            "p".to_string(),
                            HashMap::new(),
                            vec![dom::Node::text("paragrapgh".to_string())],
                        ),
                        dom::Node::elem(
                            "span".to_string(),
                            HashMap::new(),
                            vec![dom::Node::text("aa".to_string())],
                        ),
                        dom::Node::text(" space".to_string()),
                        dom::Node::elem(
                            "img".to_string(),
                            {
                                let mut h = HashMap::new();
                                h.insert("src".to_string(), "./a/a.png".to_string());
                                h
                            },
                            vec![],
                        ),
                    ],
                ),
            ]
        )
    );
}

#[test]
fn test_empty_source() {
    use std::path::Path;
    let src = "";
    let dom_node = parse(src.to_string(), Path::new("a.html").to_path_buf());
    assert_eq!(
        dom_node,
        dom::Node::elem("html".to_string(), HashMap::new(), vec![])
    );
}
