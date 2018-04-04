use dom::{ElementData, Node, NodeType};
use css::{parse_attr_style, Declaration, Rule, Selector, SimpleSelector, Specificity, Stylesheet,
          Value};
use std::collections::HashMap;

pub type PropertyMap = HashMap<String, Vec<Value>>;

#[derive(Clone, Debug)]
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

#[derive(PartialEq, Debug)]
pub enum Display {
    Inline,
    Block,
    InlineBlock,
    None,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum FloatType {
    Left,
    Right,
    None,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum ClearType {
    Left,
    Right,
    Both,
}

impl<'a> StyledNode<'a> {
    pub fn value(&self, name: &str) -> Option<Vec<Value>> {
        self.specified_values.get(name).cloned()
    }

    pub fn value_with_default(&self, name: &str, default: &Vec<Value>) -> Vec<Value> {
        self.value(name).unwrap_or(default.clone())
    }

    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Vec<Value>) -> Vec<Value> {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    pub fn lookup_without_default(&self, name: &str, fallback_name: &str) -> Option<Vec<Value>> {
        self.value(name).or_else(|| self.value(fallback_name))
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "block" => Display::Block,
                    "inline-block" => Display::InlineBlock,
                    "none" => Display::None,
                    "inline" | _ => Display::Inline,
                },
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }

    pub fn float(&self) -> FloatType {
        match self.value("float") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "left" => FloatType::Left,
                    "right" => FloatType::Right,
                    "none" => FloatType::None,
                    _ => FloatType::None,
                },
                _ => FloatType::None,
            },
            _ => FloatType::None,
        }
    }

    pub fn clear(&self) -> Option<ClearType> {
        match self.value("clear") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "left" => Some(ClearType::Left),
                    "right" => Some(ClearType::Right),
                    "both" => Some(ClearType::Both),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    pub fn has_text_node(&self) -> bool {
        match self.node.data {
            NodeType::Text(_) => true,
            _ => false,
        }
    }
}

fn inherit_peoperties(specified_values: &PropertyMap, property_list: Vec<&str>) -> PropertyMap {
    let mut inherited_property = PropertyMap::new();
    for property in property_list {
        if let Some(value) = specified_values.get(property) {
            inherited_property.insert(property.to_string(), value.clone());
        }
    }
    inherited_property
}

pub fn style_tree<'a>(
    root: &'a Node,
    stylesheet: &'a Stylesheet,
    inherited_property: &PropertyMap,
    mut appeared_elements: Vec<SimpleSelector>,
) -> StyledNode<'a> {
    let specified_values = match root.data {
        NodeType::Element(ref elem) => {
            appeared_elements.push(SimpleSelector {
                tag_name: Some(elem.tag_name.clone()),
                id: elem.id().and_then(|id| Some(id.clone())),
                class: elem.classes().iter().map(|x| x.to_string()).collect(),
            });
            specified_values(
                elem,
                stylesheet,
                inherited_property,
                appeared_elements.clone(),
            )
        }
        // TODO: Fix this implementation
        NodeType::Text(_) => inherited_property.clone(),
    };

    let inherited_property = inherit_peoperties(
        &specified_values,
        vec![
            "font-size",
            "line-height",
            "font-weight",
            "font-style",
            "text-align",
            "color",
        ],
    );

    StyledNode {
        node: root,
        children: root.children
            .iter()
            .map(|child| {
                style_tree(
                    child,
                    stylesheet,
                    &inherited_property,
                    appeared_elements.clone(),
                )
            })
            .collect(),
        specified_values: specified_values,
    }
}

fn specified_values(
    elem: &ElementData,
    stylesheet: &Stylesheet,
    inherited_property: &PropertyMap,
    appeared_elements: Vec<SimpleSelector>,
) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet, appeared_elements);

    // Insert inherited properties
    inherited_property.iter().for_each(|(name, value)| {
        values.insert(name.clone(), value.clone());
    });

    // Go through the rules from lowest to highest specificity.
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    rules.iter().for_each(|&(_, rule)| {
        rule.declarations.iter().for_each(|declaration| {
            values.insert(declaration.name.clone(), declaration.values.clone());
        })
    });

    if let Some(attr_style) = elem.attrs.get("style") {
        let decls = parse_attr_style(attr_style.clone());
        decls.iter().for_each(
            |&Declaration {
                 ref name,
                 values: ref vals,
             }| {
                values.insert(name.clone(), vals.clone());
            },
        );
    }

    values
}

type MatchedRule<'a> = (Specificity, &'a Rule);

fn matching_rules<'a>(
    elem: &ElementData,
    stylesheet: &'a Stylesheet,
    appeared_elements: Vec<SimpleSelector>,
) -> Vec<MatchedRule<'a>> {
    // For now, we just do a linear scan of all the rules.  For large
    // documents, it would be more efficient to store the rules in hash tables
    // based on tag name, id, class, etc.
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule, appeared_elements.clone()))
        .collect()
}

fn match_rule<'a>(
    elem: &ElementData,
    rule: &'a Rule,
    appeared_elements: Vec<SimpleSelector>,
) -> Option<MatchedRule<'a>> {
    // Find the first (most specific) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, *selector, appeared_elements.clone()))
        .map(|selector| (selector.specificity(), rule))
}

fn matches(
    elem: &ElementData,
    selector: &Selector,
    appeared_elements: Vec<SimpleSelector>,
) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
        Selector::Descendant(ref a, ref b) => {
            matches_descendant_combinator(elem, &**a, &**b, appeared_elements)
        }
    }
}

fn matches_descendant_combinator(
    elem: &ElementData,
    selector_a: &Selector,
    selector_b: &Selector,
    appeared_elements: Vec<SimpleSelector>,
) -> bool {
    if let &Selector::Simple(ref simple) = selector_a {
        appeared_elements
            .iter()
            .rev()
            .any(|e| e.tag_name == simple.tag_name && e.class.is_superset(&simple.class))
            && matches(elem, selector_b, appeared_elements)
    } else {
        unreachable!()
    }
}

fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    // Check type selector
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }

    // Check ID selector
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    // Check class selectors
    let elem_classes = elem.classes();
    if selector
        .class
        .iter()
        .any(|class| !elem_classes.contains(&**class))
    {
        return false;
    }

    // We didn't find any non-matching selector components.
    true
}

#[test]
fn test1() {
    use html;
    use css;
    use std::path::Path;

    let src = "<html>
                 <head>
                 </head>
                 <body style='font-size:10px;'>
                   <div id=\"x\">test</div>
                   <p>paragrapgh</p>
                   <span style='color:red;'>aa</span>
                   space
                 </body>
               </html>";
    let dom_node = html::parse(src.to_string(), Path::new("a.html").to_path_buf());

    let src = "div { width: 100px; height: 50px; color: #ffffff; background-color: #003300; }";
    let stylesheet = css::parse(src.to_string());

    // TODO
    style_tree(&dom_node, &stylesheet, &PropertyMap::new());
}
