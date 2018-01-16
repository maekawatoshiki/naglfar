use std::collections::{HashMap, HashSet};
use std::{fmt, iter};

pub type AttrMap = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct Node {
    // data specific to each node type:
    pub data: NodeType,
    // data common to all nodes:
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Element(ElementData),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
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
                tag_name: name,
                attrs: attrs,
            }),
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
