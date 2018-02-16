use dom::{ElementData, Node, NodeType};
use css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value};
use std::collections::HashMap;

pub type PropertyMap = HashMap<String, Value>;

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
    None,
}

impl<'a> StyledNode<'a> {
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
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
    parent_property: &PropertyMap,
) -> StyledNode<'a> {
    let specified_values = match root.data {
        NodeType::Element(ref elem) => specified_values(elem, stylesheet, inherited_property),
        // TODO: Fix this implementation
        NodeType::Text(_) => inherit_peoperties(
            &parent_property,
            vec![
                "background",
                "color",
                "border-color",
                "border-style",
                "border-size",
                "font-size",
                "line-height",
                "font-weight",
            ],
        ),
    };

    let inherited_property = inherit_peoperties(
        &specified_values,
        vec!["font-size", "line-height", "font-weight"],
    );

    StyledNode {
        node: root,
        children: root.children
            .iter()
            .map(|child| style_tree(child, stylesheet, &inherited_property, &specified_values))
            .collect(),
        specified_values: specified_values,
    }
}

fn specified_values(
    elem: &ElementData,
    stylesheet: &Stylesheet,
    inherited_property: &PropertyMap,
) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    // Insert inherited properties
    for (name, value) in inherited_property {
        values.insert(name.clone(), value.clone());
    }

    // Go through the rules from lowest to highest specificity.
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

type MatchedRule<'a> = (Specificity, &'a Rule);

fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    // For now, we just do a linear scan of all the rules.  For large
    // documents, it would be more efficient to store the rules in hash tables
    // based on tag name, id, class, etc.
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    // Find the first (most specific) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, *selector))
        .map(|selector| (selector.specificity(), rule))
}

fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
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

    let src = "<html><head></head><body><div id=\"x\">test</div><p>paragrapgh</p><span>aa</span>\n  space</body></html>";
    let dom_node = html::parse(src.to_string());

    let src = "div { width: 100px; height: 50px; color: #ffffff; background-color: #003300; }";
    let stylesheet = css::parse(src.to_string());

    // TODO
    style_tree(
        &dom_node,
        &stylesheet,
        &PropertyMap::new(),
        &PropertyMap::new(),
    );
}
