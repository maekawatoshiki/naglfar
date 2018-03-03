use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{fmt, iter};
use css;

pub type AttrMap = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    // data specific to each node type:
    pub data: NodeType,
    // data common to all nodes:
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Element(ElementData),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutType {
    Image,
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementData {
    pub tag_name: String,
    pub layout_type: LayoutType,
    pub attrs: AttrMap,
}

impl Node {
    pub fn text(data: String) -> Node {
        Node {
            children: vec![],
            data: NodeType::Text(data),
        }
    }

    pub fn elem(name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
        Node {
            children: children,
            data: NodeType::Element(ElementData {
                layout_type: match name.to_lowercase().as_str() {
                    "img" => LayoutType::Image,
                    _ => LayoutType::Generic,
                },
                tag_name: name,
                attrs: attrs,
            }),
        }
    }

    pub fn contains_text(&self) -> bool {
        match self.data {
            NodeType::Element(_) => {
                for child in &self.children {
                    if child.contains_text() {
                        return true;
                    }
                }
                false
            }
            NodeType::Text(_) => true,
        }
    }

    pub fn layout_type(&self) -> LayoutType {
        match self.data {
            NodeType::Element(ElementData {
                ref layout_type, ..
            }) => layout_type.clone(),
            NodeType::Text(_) => LayoutType::Generic,
        }
    }

    pub fn is_inline(&self) -> bool {
        match self.data {
            NodeType::Element(ElementData { ref tag_name, .. }) => match tag_name.as_str() {
                "a" | "abbr" | "acronym" | "b" | "bdo" | "big" | "br" | "button" | "cite"
                | "code" | "dfn" | "em" | "i" | "img" | "input" | "kbd" | "label" | "map"
                | "object" | "q" | "samp" | "script" | "select" | "small" | "span" | "strong"
                | "sub" | "sup" | "textarea" | "time" | "tt" | "var" => true,
                _ => false,
            },
            NodeType::Text(_) => false,
        }
    }

    pub fn find_first_node_by_tag_name<'a>(&'a self, expected: &str) -> Option<&'a Node> {
        match self.data {
            NodeType::Element(ElementData { ref tag_name, .. }) if expected == tag_name => {
                Some(self)
            }
            _ => self.children
                .iter()
                .find(|child| child.find_first_node_by_tag_name(expected).is_some()),
        }
    }

    pub fn find_stylesheet_path(&self) -> Option<PathBuf> {
        self.find_first_node_by_tag_name("link")
            .and_then(|&Node { ref data, .. }| match data {
                &NodeType::Element(ElementData { ref attrs, .. }) => attrs
                    .get("href")
                    .and_then(|filename| Some(Path::new(filename).to_path_buf())),
                &NodeType::Text(_) => None,
            })
    }

    pub fn image_url(&self) -> Option<&String> {
        match self.data {
            NodeType::Element(ElementData { ref attrs, .. }) => attrs.get("src"),
            NodeType::Text(_) => None,
        }
    }

    pub fn attr_width(&self) -> Option<css::Value> {
        match self.data {
            NodeType::Element(ElementData { ref attrs, .. }) => attrs
                .get("width")
                .and_then(|width| Some(css::parse_value(width.clone()))),
            NodeType::Text(_) => None,
        }
    }
    pub fn attr_height(&self) -> Option<css::Value> {
        match self.data {
            NodeType::Element(ElementData { ref attrs, .. }) => attrs
                .get("height")
                .and_then(|width| Some(css::parse_value(width.clone()))),
            NodeType::Text(_) => None,
        }
    }
}

// Element methods

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attrs.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attrs.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new(),
        }
    }
}

// Functions for displaying

fn walk(node: &Node, indent: usize, f: &mut fmt::Formatter) -> fmt::Result {
    try!(write!(
        f,
        "{}",
        iter::repeat(" ").take(indent).collect::<String>()
    ));
    try!(write!(f, "{}\n", node.data));
    for child in &node.children {
        try!(walk(child, indent + 2, f));
    }
    Ok(())
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        walk(self, 0, f)
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &NodeType::Element(ElementData {
                ref tag_name,
                ref attrs,
                ..
            }) => {
                try!(write!(f, "<{}", tag_name));
                for (name, val) in attrs.iter() {
                    try!(write!(f, " {}=\"{}\"", name, val));
                }
                write!(f, ">")
            }
            &NodeType::Text(ref body) => write!(f, "#text: {}", escape_default(body.as_str())),
        }
    }
}

fn escape_default(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.escape_default())
        .collect::<String>()
}
