use dom::{ElementData, Node, NodeType};
use css::{parse_attr_style, Color, Declaration, Rule, Selector, SimpleSelector, Specificity,
          Stylesheet, TextDecoration, Unit, Value};
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

    pub fn padding(&self) -> (Value, Value, Value, Value) {
        // padding has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut padding_top = self.value("padding-top").and_then(|x| Some(x[0].clone()));
        let mut padding_bottom = self.value("padding-bottom")
            .and_then(|x| Some(x[0].clone()));
        let mut padding_left = self.value("padding-left").and_then(|x| Some(x[0].clone()));
        let mut padding_right = self.value("padding-right").and_then(|x| Some(x[0].clone()));

        if let Some(padding) = self.value("padding") {
            match padding.len() {
                1 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_bottom.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[0].clone());
                    padding_right.get_or_insert_with(|| padding[0].clone());
                }
                2 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_bottom.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[1].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                }
                3 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[1].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                    padding_bottom.get_or_insert_with(|| padding[2].clone());
                }
                4 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                    padding_bottom.get_or_insert_with(|| padding[2].clone());
                    padding_left.get_or_insert_with(|| padding[3].clone());
                }
                0 | _ => unreachable!(),
            }
        }

        padding_top.get_or_insert_with(|| zero.clone());
        padding_right.get_or_insert_with(|| zero.clone());
        padding_bottom.get_or_insert_with(|| zero.clone());
        padding_left.get_or_insert_with(|| zero.clone());

        (
            padding_top.unwrap(),
            padding_right.unwrap(),
            padding_bottom.unwrap(),
            padding_left.unwrap(),
        )
    }

    pub fn margin(&self) -> (Value, Value, Value, Value) {
        // margin has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_top = self.value("margin-top").and_then(|x| Some(x[0].clone()));
        let mut margin_bottom = self.value("margin-bottom").and_then(|x| Some(x[0].clone()));
        let mut margin_left = self.value("margin-left").and_then(|x| Some(x[0].clone()));
        let mut margin_right = self.value("margin-right").and_then(|x| Some(x[0].clone()));

        if let Some(margin) = self.value("margin") {
            match margin.len() {
                1 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_bottom.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[0].clone());
                    margin_right.get_or_insert_with(|| margin[0].clone());
                }
                2 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_bottom.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[1].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                }
                3 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[1].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                    margin_bottom.get_or_insert_with(|| margin[2].clone());
                }
                4 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                    margin_bottom.get_or_insert_with(|| margin[2].clone());
                    margin_left.get_or_insert_with(|| margin[3].clone());
                }
                0 | _ => unreachable!(),
            }
        }

        margin_top.get_or_insert_with(|| zero.clone());
        margin_right.get_or_insert_with(|| zero.clone());
        margin_bottom.get_or_insert_with(|| zero.clone());
        margin_left.get_or_insert_with(|| zero.clone());

        (
            margin_top.unwrap(),
            margin_right.unwrap(),
            margin_bottom.unwrap(),
            margin_left.unwrap(),
        )
    }

    pub fn border_width(&self) -> (Value, Value, Value, Value) {
        // border has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut border_top = self.value("border-top-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_bottom = self.value("border-bottom-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_left = self.value("border-left-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_right = self.value("border-right-width")
            .and_then(|x| Some(x[0].clone()));

        if let Some(border) = self.value("border-width") {
            match border.len() {
                1 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_bottom.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[0].clone());
                    border_right.get_or_insert_with(|| border[0].clone());
                }
                2 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_bottom.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[1].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                }
                3 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[1].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                    border_bottom.get_or_insert_with(|| border[2].clone());
                }
                4 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                    border_bottom.get_or_insert_with(|| border[2].clone());
                    border_left.get_or_insert_with(|| border[3].clone());
                }
                0 | _ => unreachable!(),
            }
        } else if let Some(border_info) = self.value("border") {
            let mut border_width = None;
            for border in border_info {
                if let &Value::Length(_, _) = &border {
                    border_width = Some(border);
                    break;
                }
            }
            if let Some(border_width) = border_width {
                border_top.get_or_insert_with(|| border_width.clone());
                border_right.get_or_insert_with(|| border_width.clone());
                border_bottom.get_or_insert_with(|| border_width.clone());
                border_left.get_or_insert_with(|| border_width.clone());
            }
        }

        border_top.get_or_insert_with(|| zero.clone());
        border_right.get_or_insert_with(|| zero.clone());
        border_bottom.get_or_insert_with(|| zero.clone());
        border_left.get_or_insert_with(|| zero.clone());

        (
            border_top.unwrap(),
            border_right.unwrap(),
            border_bottom.unwrap(),
            border_left.unwrap(),
        )
    }

    pub fn border_color(&self) -> (Option<Color>, Option<Color>, Option<Color>, Option<Color>) {
        let mut border_top = self.value("border-top-color").and_then(|x| x[0].to_color());
        let mut border_bottom = self.value("border-bottom-color")
            .and_then(|x| x[0].to_color());
        let mut border_left = self.value("border-left-color")
            .and_then(|x| x[0].to_color());
        let mut border_right = self.value("border-right-color")
            .and_then(|x| x[0].to_color());

        if let Some(border) = self.value("border-color") {
            match border.len() {
                1 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[0].to_color().unwrap());
                }
                2 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                }
                3 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[2].to_color().unwrap());
                }
                4 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[2].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[3].to_color().unwrap());
                }
                0 | _ => unreachable!(),
            }
        } else if let Some(border_info) = self.value("border") {
            if let Some(border_color) = (|| {
                for border in border_info {
                    let color = border.to_color();
                    if color.is_some() {
                        return color;
                    }
                }
                None
            })()
            {
                border_top.get_or_insert_with(|| border_color.clone());
                border_right.get_or_insert_with(|| border_color.clone());
                border_bottom.get_or_insert_with(|| border_color.clone());
                border_left.get_or_insert_with(|| border_color.clone());
            }
        }

        (border_top, border_right, border_bottom, border_left)
    }

    pub fn text_decoration(&self) -> Vec<TextDecoration> {
        if let Some(text_decorations) = self.value("text-decoration") {
            let mut decorations = vec![];
            for text_decoration in text_decorations {
                if let Some(decoration) = text_decoration.to_text_decoration() {
                    decorations.push(decoration);
                }
            }
            decorations
        } else {
            vec![]
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
    parent_specified_values: &PropertyMap,
    appeared_elements: &Vec<SimpleSelector>,
) -> StyledNode<'a> {
    let mut appeared_elements = appeared_elements.clone();

    let specified_values = match root.data {
        NodeType::Element(ref elem) => {
            let values = specified_values(elem, stylesheet, inherited_property, &appeared_elements);
            appeared_elements.push(SimpleSelector {
                tag_name: Some(elem.tag_name.clone()),
                id: elem.id().and_then(|id| Some(id.clone())),
                class: elem.classes().iter().map(|x| x.to_string()).collect(),
            });
            values
        }
        NodeType::Text(_) => {
            if let Some(display) = parent_specified_values.get("display") {
                match display[0] {
                    // If the parent element is an inline element, inherites the parent's properties.
                    Value::Keyword(ref k) if k == "inline" => parent_specified_values.clone(),
                    _ => inherited_property.clone(),
                }
            } else {
                unreachable!()
            }
        }
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
                    &specified_values,
                    &appeared_elements,
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
    appeared_elements: &Vec<SimpleSelector>,
) -> PropertyMap {
    let mut values = HashMap::with_capacity(16);
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
        for Declaration { name, values: vals } in decls {
            values.insert(name, vals);
        }
    }

    values
}

type MatchedRule<'a> = (Specificity, &'a Rule);

fn matching_rules<'a>(
    elem: &ElementData,
    stylesheet: &'a Stylesheet,
    appeared_elements: &Vec<SimpleSelector>,
) -> Vec<MatchedRule<'a>> {
    // For now, we just do a linear scan of all the rules.  For large
    // documents, it would be more efficient to store the rules in hash tables
    // based on tag name, id, class, etc.
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule, appeared_elements))
        .collect()
}

fn match_rule<'a>(
    elem: &ElementData,
    rule: &'a Rule,
    appeared_elements: &Vec<SimpleSelector>,
) -> Option<MatchedRule<'a>> {
    // Find the first (most specific) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, *selector, appeared_elements))
        .map(|selector| (selector.specificity(), rule))
}

fn matches(
    elem: &ElementData,
    selector: &Selector,
    appeared_elements: &Vec<SimpleSelector>,
) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
        Selector::Descendant(ref a, ref b) => {
            matches_descendant_combinator(elem, &*a, &**b, appeared_elements)
        }
        Selector::Child(ref a, ref b) => {
            matches_child_combinator(elem, &*a, &**b, appeared_elements)
        }
    }
}

fn matches_descendant_combinator(
    elem: &ElementData,
    simple: &SimpleSelector,
    selector_b: &Selector,
    appeared_elements: &Vec<SimpleSelector>,
) -> bool {
    appeared_elements.iter().rev().any(|e| {
        e.tag_name == simple.tag_name && e.id == simple.id
            && !simple.class.iter().any(|class| !e.class.contains(class))
    }) && matches(elem, selector_b, appeared_elements)
}

fn matches_child_combinator(
    elem: &ElementData,
    simple: &SimpleSelector,
    selector_b: &Selector,
    appeared_elements: &Vec<SimpleSelector>,
) -> bool {
    if let Some(ref last_elem) = appeared_elements.last() {
        last_elem.tag_name == simple.tag_name && last_elem.id == simple.id
            && !simple
                .class
                .iter()
                .any(|class| !last_elem.class.contains(class))
            && matches(elem, selector_b, appeared_elements)
    } else {
        false
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
                   <a>link</a>
                   space
                 </body>
               </html>";
    let dom_node = html::parse(src.to_string(), Path::new("a.html").to_path_buf());

    let src = "* { display: block; }
               div, body > div, body span { width: 100px; height: 50px; color: #ffffff; background-color: #003300; } 
               a { display: inline; text-decoration: underline; }";
    let stylesheet = css::parse(src.to_string());

    // TODO
    style_tree(
        &dom_node,
        &stylesheet,
        &PropertyMap::new(),
        &PropertyMap::new(),
        &vec![],
    );
}
