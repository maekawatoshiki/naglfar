use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::{LayoutType, NodeType};
use float::Floats;
use font::{Font, FontSlant, FontWeight};
use inline::LineMaker;
use style;

use std::default::Default;
use std::fmt;
use std::ops::Range;

use cairo;
use pango;
use gdk_pixbuf;

use app_units::Au;

// CSS box model. All sizes are in px.
// TODO: Support units other than px

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
    Image(Option<gdk_pixbuf::Pixbuf>),
    Anker,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoxType {
    BlockNode,
    InlineNode,
    InlineBlockNode,
    Float,
    TextNode(Text),
    AnonymousBlock,
}

// A node in the layout tree.
#[derive(Clone, Debug)]
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub z_index: i32,
    pub box_type: BoxType,
    pub info: LayoutInfo,
    pub floats: Floats,
    pub style: Option<&'a StyledNode<'a>>,
    pub children: Vec<LayoutBox<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub font: Font,
    pub range: Range<usize>,
}

pub type Texts = Vec<Text>;

impl<'a> LayoutBox<'a> {
    pub fn new(
        box_type: BoxType,
        style: Option<&'a StyledNode<'a>>,
        info: LayoutInfo,
    ) -> LayoutBox<'a> {
        LayoutBox {
            box_type: box_type,
            style: style,
            info: info,
            z_index: 0,
            floats: Floats::new(),
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    pub fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.style {
            Some(style) => style,
            None => panic!(),
        }
    }

    pub fn set_text_info(&mut self, font: Font, range: Range<usize>) {
        if let BoxType::TextNode(ref mut r) = self.box_type {
            r.font = font;
            r.range = range;
        }
    }
}

pub const DEFAULT_FONT_SIZE: f64 = 16.0f64;
pub const DEFAULT_LINE_HEIGHT: f64 = DEFAULT_FONT_SIZE * 1.2f64;

/// Transform a style tree into a layout tree.
pub fn layout_tree<'a>(
    node: &'a StyledNode<'a>,
    mut containing_block: Dimensions,
) -> LayoutBox<'a> {
    // Save the initial containing block height for calculating percent heights.
    let saved_block = containing_block.clone();
    let viewport = containing_block.clone();
    // The layout algorithm expects the container height to start at 0.
    containing_block.content.height = Au::from_f64_px(0.0);

    let mut root_box = build_layout_tree(node);
    root_box.layout(
        &mut Floats::new(),
        Au(0),
        containing_block,
        saved_block,
        viewport,
    );
    root_box
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(
        match style_node.display() {
            Display::Block => BoxType::BlockNode,
            Display::Inline => match style_node.node.data {
                NodeType::Element(_) => BoxType::InlineNode,
                NodeType::Text(ref s) => BoxType::TextNode(Text {
                    font: Font::new_empty(),
                    range: 0..s.len(),
                }),
            },
            Display::InlineBlock => match style_node.node.data {
                NodeType::Element(_) => BoxType::InlineBlockNode,
                NodeType::Text(_) => panic!(),
            },
            Display::None => panic!("Root node has display: none."),
        },
        Some(style_node),
        match style_node.node.layout_type() {
            LayoutType::Generic => LayoutInfo::Generic,
            LayoutType::Text => LayoutInfo::Text,
            LayoutType::Image => LayoutInfo::Image(None),
            LayoutType::Anker => LayoutInfo::Anker,
        },
    );

    match style_node.float() {
        style::FloatType::None => {}
        style::FloatType::Left | style::FloatType::Right => root.box_type = BoxType::Float,
    }

    // Create the descendant boxes.
    let mut float_insert_point: Option<usize> = None;
    for (i, child) in style_node.children.iter().enumerate() {
        match (child.display(), child.float()) {
            (Display::Block, style::FloatType::None) => {
                root.children.push(build_layout_tree(child));
                if float_insert_point.is_some() {
                    float_insert_point = None;
                }
            }
            (Display::Inline, style::FloatType::None)
            | (Display::InlineBlock, style::FloatType::None) => {
                root.get_inline_container()
                    .children
                    .push(build_layout_tree(child));
                float_insert_point = Some(i);
            }
            (_, style::FloatType::Left) | (_, style::FloatType::Right) => {
                if let Some(pos) = float_insert_point {
                    root.children.insert(pos, build_layout_tree(child));
                } else {
                    root.children.push(build_layout_tree(child));
                }
            }
            (Display::None, _) => {} // Don't lay out nodes with `display: none;`
        }
    }

    root
}

impl<'a> LayoutBox<'a> {
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
            BoxType::AnonymousBlock => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = containing_block.content.height;

                let mut linemaker = LineMaker::new(self.children.clone(), floats.clone());
                linemaker.run(containing_block.content.width, containing_block);
                linemaker.end_of_lines();
                linemaker.assign_position(containing_block.content.width);

                self.children = linemaker.new_boxes;
                self.dimensions.content.width = containing_block.content.width;
                self.dimensions.content.height = linemaker.cur_height;
            }
            BoxType::Float => self.layout_float(
                floats,
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::InlineNode | BoxType::TextNode(_) => unreachable!(),
        }
    }

    /// Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
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
                        None,
                        LayoutInfo::Generic,
                    )),
                }
                self.children.last_mut().unwrap()
            }
            BoxType::TextNode(_) => panic!(),
        }
    }

    pub fn get_padding(&mut self) -> (Value, Value, Value, Value) {
        let style = self.get_style_node();

        // padding has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut padding_top = style.value("padding-top").and_then(|x| Some(x[0].clone()));
        let mut padding_bottom = style
            .value("padding-bottom")
            .and_then(|x| Some(x[0].clone()));
        let mut padding_left = style.value("padding-left").and_then(|x| Some(x[0].clone()));
        let mut padding_right = style
            .value("padding-right")
            .and_then(|x| Some(x[0].clone()));

        if let Some(padding) = style.value("padding") {
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

    pub fn assign_padding(&mut self) {
        let (padding_top, padding_right, padding_bottom, padding_left) = self.get_padding();

        let d = &mut self.dimensions;
        d.padding.left = Au::from_f64_px(padding_left.to_px().unwrap());
        d.padding.top = Au::from_f64_px(padding_top.to_px().unwrap());
        d.padding.bottom = Au::from_f64_px(padding_bottom.to_px().unwrap());
        d.padding.right = Au::from_f64_px(padding_right.to_px().unwrap());
    }

    pub fn get_margin(&mut self) -> (Value, Value, Value, Value) {
        let style = self.get_style_node();

        // margin has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_top = style.value("margin-top").and_then(|x| Some(x[0].clone()));
        let mut margin_bottom = style
            .value("margin-bottom")
            .and_then(|x| Some(x[0].clone()));
        let mut margin_left = style.value("margin-left").and_then(|x| Some(x[0].clone()));
        let mut margin_right = style.value("margin-right").and_then(|x| Some(x[0].clone()));

        if let Some(margin) = style.value("margin") {
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

    pub fn assign_margin(&mut self) {
        let (margin_top, margin_right, margin_bottom, margin_left) = self.get_margin();

        let d = &mut self.dimensions;
        d.margin.left = Au::from_f64_px(margin_left.to_px().unwrap());
        d.margin.top = Au::from_f64_px(margin_top.to_px().unwrap());
        d.margin.bottom = Au::from_f64_px(margin_bottom.to_px().unwrap());
        d.margin.right = Au::from_f64_px(margin_right.to_px().unwrap());
    }

    pub fn get_border_width(&mut self) -> (Value, Value, Value, Value) {
        let style = self.get_style_node();

        // border has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut border_top = style
            .value("border-top-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_bottom = style
            .value("border-bottom-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_left = style
            .value("border-left-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_right = style
            .value("border-right-width")
            .and_then(|x| Some(x[0].clone()));

        if let Some(border) = style.value("border-width") {
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

    pub fn assign_border_width(&mut self) {
        let (border_top, border_right, border_bottom, border_left) = self.get_border_width();

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

impl Value {
    pub fn to_font_weight(&self) -> FontWeight {
        match self {
            &Value::Keyword(ref k) if k.as_str() == "normal" => FontWeight::Normal,
            &Value::Keyword(ref k) if k.as_str() == "bold" => FontWeight::Bold,
            _ => FontWeight::Normal,
        }
    }
    pub fn to_font_slant(&self) -> FontSlant {
        match self {
            &Value::Keyword(ref k) if k.as_str() == "normal" => FontSlant::Normal,
            &Value::Keyword(ref k) if k.as_str() == "italic" => FontSlant::Italic,
            _ => FontSlant::Normal,
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
impl<'a> fmt::Display for LayoutBox<'a> {
    // TODO: Implement all features
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:?}", self.dimensions)?;
        for child in &self.children {
            write!(f, "{}", child)?;
        }
        Ok(())
    }
}
