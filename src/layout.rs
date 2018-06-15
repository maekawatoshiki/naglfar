use style::{Display, Style};
use dom::{ElementData, LayoutType, Node, NodeType};
use float::Floats;
use font::{Font, FontSlant, FontWeight};
use inline::LineMaker;
use style;
use default_style;
use css::{parse_attr_style, Declaration, Rule, Selector, SimpleSelector, Specificity, Stylesheet,
          Value};

use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::ops::Range;

use cairo;
use pango;
use gdk_pixbuf;
use gtk;

use app_units::Au;

// CSS box model. All sizes are in px.

#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq)]
pub struct Rect {
    pub x: Au,
    pub y: Au,
    pub width: Au,
    pub height: Au,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    // Position of the content area relative to the document origin:
    pub content: Rect,
    // Surrounding edges:
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EdgeSizes {
    pub left: Au,
    pub right: Au,
    pub top: Au,
    pub bottom: Au,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutInfo {
    Generic,
    Text,
    Image(ImageData),
    Anker,
    Button(Option<gtk::Button>, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageData {
    pub pixbuf: Option<gdk_pixbuf::Pixbuf>,
    pub metadata: ImageMetaData,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageMetaData {
    pub width: Au,
    pub height: Au,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoxType {
    BlockNode,
    InlineNode,
    InlineBlockNode,
    Float,
    TextNode(Text),
    AnonymousBlock,
    None, // TODO: Is this really needed?
}

// A node in the layout tree.
#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub node: Node,
    pub property: Style,
    pub dimensions: Dimensions,
    pub z_index: i32,
    pub box_type: BoxType,
    pub info: LayoutInfo,
    pub floats: Floats,
    pub children: Vec<LayoutBox>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub font: Font,
    pub range: Range<usize>,
}

impl ImageData {
    pub fn new(pixbuf: Option<gdk_pixbuf::Pixbuf>, metadata: ImageMetaData) -> ImageData {
        ImageData {
            pixbuf: pixbuf,
            metadata: metadata,
        }
    }

    pub fn new_empty() -> ImageData {
        ImageData::new(None, ImageMetaData::new(Au(0), Au(0)))
    }
}

impl ImageMetaData {
    pub fn new(width: Au, height: Au) -> ImageMetaData {
        ImageMetaData {
            width: width,
            height: height,
        }
    }
}

impl LayoutBox {
    pub fn new(box_type: BoxType, node: Node, property: Style, info: LayoutInfo) -> LayoutBox {
        LayoutBox {
            node: node,
            property: property,
            box_type: box_type,
            info: info,
            z_index: 0,
            floats: Floats::new(),
            dimensions: Default::default(),
            children: Vec::with_capacity(16),
        }
    }

    pub fn get_style_node(&self) -> &Style {
        &self.property
    }

    pub fn set_text_info(&mut self, font: Font, range: Range<usize>) {
        if let BoxType::TextNode(ref mut r) = self.box_type {
            r.font = font;
            r.range = range;
        }
    }

    pub fn in_normal_flow(&self) -> bool {
        self.box_type != BoxType::Float
    }
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree(
    node: &Node,
    stylesheet: &Stylesheet,
    default_style: &Stylesheet,
    inherited_property: &Style,
    parent_specified_values: &Style,
    appeared_elements: &Vec<SimpleSelector>,
    id: &mut usize,
) -> LayoutBox {
    let mut appeared_elements = appeared_elements.clone();
    let specified_values = match node.data {
        NodeType::Element(ref elem) => {
            let values = specified_values(
                elem,
                default_style,
                stylesheet,
                inherited_property,
                &appeared_elements,
            );
            appeared_elements.push(SimpleSelector {
                tag_name: Some(elem.tag_name.clone()),
                id: elem.id().and_then(|id| Some(id.clone())),
                class: elem.classes().iter().map(|x| x.to_string()).collect(),
            });
            values
        }
        NodeType::Text(_) => {
            Style::new_with(
                if let Some(display) = parent_specified_values.property.get("display") {
                    match display[0] {
                        // If the parent element is an inline element, inherites the parent's properties.
                        Value::Keyword(ref k) if k == "inline" => parent_specified_values.clone(),
                        _ => inherited_property.clone(),
                    }
                } else {
                    inherited_property.clone()
                }.property
                    .into_iter()
                    .filter(|&(ref name, _)| name != "float")
                    .collect(),
            )
        }
    };

    // Create the root box.
    let mut root = LayoutBox::new(
        match specified_values.display() {
            Display::Block => BoxType::BlockNode,
            Display::Inline => match node.data {
                NodeType::Element(_) => BoxType::InlineNode,
                NodeType::Text(ref s) => BoxType::TextNode(Text {
                    font: Font::new_empty(),
                    range: 0..s.len(),
                }),
            },
            Display::InlineBlock => match node.data {
                NodeType::Element(_) => BoxType::InlineBlockNode,
                NodeType::Text(_) => panic!(),
            },
            Display::None => BoxType::None, // TODO
        },
        node.clone(),
        specified_values.clone(),
        match node.layout_type() {
            LayoutType::Generic => LayoutInfo::Generic,
            LayoutType::Text => LayoutInfo::Text,
            LayoutType::Image => LayoutInfo::Image(ImageData::new_empty()),
            LayoutType::Anker => LayoutInfo::Anker,
            LayoutType::Button => LayoutInfo::Button(None, *id),
        },
    );

    if root.box_type == BoxType::None {
        return root;
    }

    match specified_values.float() {
        style::FloatType::None => {}
        style::FloatType::Left | style::FloatType::Right => root.box_type = BoxType::Float,
    }

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

    // Create the descendant boxes.
    let mut float_insert_point: Option<usize> = None;
    for (i, child) in node.children.iter().enumerate() {
        *id += 1;
        let child = build_layout_tree(
            child,
            stylesheet,
            default_style,
            &inherited_property,
            &specified_values,
            &appeared_elements,
            id,
        );
        match (child.property.display(), child.property.float()) {
            (Display::Block, style::FloatType::None) => {
                root.children.push(child);
                if float_insert_point.is_some() {
                    float_insert_point = None;
                }
            }
            (Display::Inline, style::FloatType::None)
            | (Display::InlineBlock, style::FloatType::None) => {
                root.get_inline_container().children.push(child);
                float_insert_point = Some(i);
            }
            (Display::None, _) => {} // Don't lay out nodes with `display: none;`
            (_, style::FloatType::Left) | (_, style::FloatType::Right) => {
                // if let Some(pos) = float_insert_point {
                //     root.children.insert(pos, build_layout_tree(child, id));
                // } else {
                root.children.push(child);
                // }
            }
        }
    }

    root
}

fn inherit_peoperties(specified_values: &Style, property_list: Vec<&str>) -> Style {
    let mut inherited_property = HashMap::new();
    let specified_values = &specified_values.property;
    for property in property_list {
        if let Some(value) = specified_values.get(property) {
            inherited_property.insert(property.to_string(), value.clone());
        }
    }
    Style::new_with(inherited_property)
}

fn specified_values(
    elem: &ElementData,
    default_style: &Stylesheet,
    stylesheet: &Stylesheet,
    inherited_property: &Style,
    appeared_elements: &Vec<SimpleSelector>,
) -> Style {
    let mut values = HashMap::with_capacity(16);

    let mut rules = matching_rules(elem, &default_style, appeared_elements);
    rules.append(&mut matching_rules(elem, stylesheet, appeared_elements));

    // Insert inherited properties
    inherited_property
        .property
        .iter()
        .for_each(|(name, value)| {
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

    Style::new_with(values)
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
    appeared_elements.iter().any(|e| {
        !((simple.tag_name.is_some() && e.tag_name != simple.tag_name)
            || (simple.id.is_some() && e.id != simple.id)
            || (!simple.class.iter().all(|class| e.class.contains(class))))
    }) && matches(elem, selector_b, appeared_elements)
}

fn matches_child_combinator(
    elem: &ElementData,
    simple: &SimpleSelector,
    selector_b: &Selector,
    appeared_elements: &Vec<SimpleSelector>,
) -> bool {
    if let Some(ref last_elem) = appeared_elements.last() {
        !((simple.tag_name.is_some() && last_elem.tag_name != simple.tag_name)
            || (simple.id.is_some() && last_elem.id != simple.id)
            || (!simple
                .class
                .iter()
                .all(|class| last_elem.class.contains(class))))
            && matches(elem, selector_b, appeared_elements)
    } else {
        false
    }
}

fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    // Universal selector
    if selector.tag_name.is_none() && selector.id.is_none() && selector.class.is_empty() {
        return true;
    }

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

use std::cell::RefCell;
thread_local!(pub static LAYOUTBOX: RefCell<Option<LayoutBox>> = { RefCell::new(None) };);

/// Transform a style tree into a layout tree.
pub fn layout_tree(
    root: &Node,
    stylesheet: &Stylesheet,
    mut containing_block: Dimensions,
) -> LayoutBox {
    let mut first_construction_of_layout_tree = false;
    let mut root_box = LAYOUTBOX.with(|layoutbox| {
        layoutbox
            .borrow_mut()
            .get_or_insert_with(|| {
                first_construction_of_layout_tree = true;
                let mut id = 0;
                let default_style = default_style::default_style();
                build_layout_tree(
                    root,
                    &stylesheet,
                    &default_style,
                    &style::Style::new(),
                    &style::Style::new(),
                    &vec![],
                    &mut id,
                )
            })
            .clone()
    });

    // Save the initial containing block height for calculating percent heights.
    let saved_block = containing_block;
    let viewport = containing_block;
    // The layout algorithm expects the container height to start at 0.
    containing_block.content.height = Au::from_f64_px(0.0);

    root_box.layout(
        &mut Floats::new(),
        Au(0),
        containing_block,
        saved_block,
        viewport,
    );

    if first_construction_of_layout_tree {
        LAYOUTBOX.with(|layoutbox| {
            if let Some(ref mut layoutbox) = *layoutbox.borrow_mut() {
                assign_style_properties(&root_box, layoutbox);
                fn assign_style_properties(root_box: &LayoutBox, layoutbox: &mut LayoutBox) {
                    if root_box.box_type != BoxType::AnonymousBlock {
                        layoutbox.property = root_box.property.clone();
                        for (child, layoutbox_child) in
                            root_box.children.iter().zip(&mut layoutbox.children)
                        {
                            assign_style_properties(child, layoutbox_child);
                        }
                    }
                }
            }
        });
    }

    root_box
}

impl LayoutBox {
    /// Lay out a box and its descendants.
    /// `saved_block` is used to know the maximum width/height of the box, calculate the percent
    /// width/height and so on.
    pub fn layout(
        &mut self,
        floats: &mut Floats,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        match self.box_type {
            BoxType::BlockNode => self.layout_block(
                floats,
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::InlineBlockNode => self.layout_inline_block(
                floats,
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::Float => self.layout_float(
                floats,
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::AnonymousBlock => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = containing_block.content.height;

                let mut linemaker = LineMaker::new(self.children.clone(), floats.clone());
                linemaker.run(containing_block.content.width, containing_block);
                linemaker.end_of_lines();
                linemaker.assign_position();

                self.dimensions.content.width = linemaker.calculate_width();
                self.dimensions.content.height = linemaker.cur_height;
                self.children = linemaker.new_boxes;
            }
            // InlineNode and TextNode is contained in AnonymousBlock.
            BoxType::InlineNode | BoxType::TextNode(_) => unreachable!(),
            BoxType::None => {}
        }
    }

    /// Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut LayoutBox {
        match self.box_type {
            BoxType::InlineNode | BoxType::AnonymousBlock => self,
            BoxType::Float | BoxType::BlockNode | BoxType::InlineBlockNode => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(
                        BoxType::AnonymousBlock,
                        Node::text("".to_string()),
                        Style::new(),
                        LayoutInfo::Generic,
                    )),
                }
                self.children.last_mut().unwrap()
            }
            BoxType::TextNode(_) => panic!(),
            BoxType::None => unreachable!(),
        }
    }

    pub fn assign_padding(&mut self) {
        let (padding_top, padding_right, padding_bottom, padding_left) = self.property.padding();

        let d = &mut self.dimensions;
        d.padding.left = Au::from_f64_px(padding_left.to_px().unwrap());
        d.padding.top = Au::from_f64_px(padding_top.to_px().unwrap());
        d.padding.bottom = Au::from_f64_px(padding_bottom.to_px().unwrap());
        d.padding.right = Au::from_f64_px(padding_right.to_px().unwrap());
    }

    pub fn assign_margin(&mut self) {
        let (margin_top, margin_right, margin_bottom, margin_left) = self.property.margin();

        let d = &mut self.dimensions;
        d.margin.left = Au::from_f64_px(margin_left.to_px().unwrap());
        d.margin.top = Au::from_f64_px(margin_top.to_px().unwrap());
        d.margin.bottom = Au::from_f64_px(margin_bottom.to_px().unwrap());
        d.margin.right = Au::from_f64_px(margin_right.to_px().unwrap());
    }

    pub fn assign_border_width(&mut self) {
        let (border_top, border_right, border_bottom, border_left) = self.property.border_width();

        let d = &mut self.dimensions;
        d.border.left = Au::from_f64_px(border_left.to_px().unwrap());
        d.border.top = Au::from_f64_px(border_top.to_px().unwrap());
        d.border.bottom = Au::from_f64_px(border_bottom.to_px().unwrap());
        d.border.right = Au::from_f64_px(border_right.to_px().unwrap());
    }
}

impl FontWeight {
    pub fn to_cairo_font_weight(&self) -> cairo::FontWeight {
        match self {
            &FontWeight::Normal => cairo::FontWeight::Normal,
            &FontWeight::Bold => cairo::FontWeight::Bold,
        }
    }
    pub fn to_pango_font_weight(&self) -> pango::Weight {
        match self {
            &FontWeight::Normal => pango::Weight::Normal,
            &FontWeight::Bold => pango::Weight::Bold,
        }
    }
}

impl FontSlant {
    pub fn to_cairo_font_slant(&self) -> cairo::FontSlant {
        match self {
            &FontSlant::Normal => cairo::FontSlant::Normal,
            &FontSlant::Italic => cairo::FontSlant::Italic,
        }
    }
    pub fn to_pango_font_slant(&self) -> pango::Style {
        match self {
            &FontSlant::Normal => pango::Style::Normal,
            &FontSlant::Italic => pango::Style::Italic,
        }
    }
}

impl Rect {
    pub fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
    pub fn add_parent_coordinate(self, x: Au, y: Au) -> Rect {
        Rect {
            x: self.x + x,
            y: self.y + y,
            width: self.width,
            height: self.height,
        }
    }
}

impl Dimensions {
    // The area covered by the content area plus its padding.
    pub fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    // The area covered by the content area plus padding and borders.
    pub fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    // The area covered by the content area plus padding, borders, and margin.
    pub fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }

    pub fn left_offset(self) -> Au {
        self.margin.left + self.border.left + self.padding.left
    }
    pub fn right_offset(self) -> Au {
        self.margin.right + self.border.right + self.padding.right
    }
    pub fn top_offset(self) -> Au {
        self.margin.top + self.border.top + self.padding.top
    }
    pub fn bottom_offset(self) -> Au {
        self.margin.bottom + self.border.bottom + self.padding.bottom
    }
    pub fn left_right_offset(self) -> EdgeSizes {
        EdgeSizes {
            top: Au(0),
            bottom: Au(0),
            left: self.left_offset(),
            right: self.right_offset(),
        }
    }
    pub fn offset(self) -> EdgeSizes {
        EdgeSizes {
            top: self.top_offset(),
            bottom: self.bottom_offset(),
            left: self.left_offset(),
            right: self.right_offset(),
        }
    }
}

// Functions for displaying

// TODO: Implement all features.
impl fmt::Display for LayoutBox {
    // TODO: Implement all features
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:?}", self.dimensions)?;
        for child in &self.children {
            write!(f, "{}", child)?;
        }
        Ok(())
    }
}
