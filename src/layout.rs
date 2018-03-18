use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::{LayoutType, NodeType};
use float::Floats;

use std::default::Default;
use std::fmt;
use std::ops::Range;
use std::collections::HashMap;
use font::{Font, FontSlant, FontWeight};
use inline::LineMaker;
use style;

use cairo;
use pango;
use gdk_pixbuf;

use app_units::Au;

// CSS box model. All sizes are in px.
// TODO: Support units other than px

#[derive(Clone, Copy, Default, Debug)]
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
    Image(gdk_pixbuf::Pixbuf),
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

// Transform a style tree into a layout tree.
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

use std::cell::RefCell;

thread_local!(
    pub static IMG_CACHE: RefCell<HashMap<(String, i32, i32), gdk_pixbuf::Pixbuf>> = {
        RefCell::new(HashMap::new())
    };
);

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
            LayoutType::Image => LayoutInfo::Image(IMG_CACHE.with(|c| {
                let mut c = c.borrow_mut();
                let image_url = style_node.node.image_url().unwrap();
                // If 'width' is specified, use its value. Otherwise, -1.
                let specified_width_px = style_node
                    .node
                    .attr("width")
                    .and_then(|w| Some(w.to_px().unwrap_or(-1.0) as i32))
                    .unwrap_or(-1);
                // The same as above
                let specified_height_px = style_node
                    .node
                    .attr("height")
                    .and_then(|h| Some(h.to_px().unwrap_or(-1.0) as i32))
                    .unwrap_or(-1);
                c.entry((image_url.clone(), specified_width_px, specified_height_px))
                    .or_insert_with(|| {
                        gdk_pixbuf::Pixbuf::new_from_file_at_scale(
                            image_url.as_str(),
                            specified_width_px,
                            specified_height_px,
                            // Preserve scale if at least one of width and height is -1.
                            specified_width_px == -1 || specified_height_px == -1,
                        ).unwrap()
                    })
                    .clone()
            })),
        },
    );

    match style_node.float() {
        style::FloatType::None => {}
        _ => root.box_type = BoxType::Float,
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
                linemaker.run(containing_block.content.width);
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

    /// Lay out a inline-block-level element and its descendants.
    fn layout_inline_block(
        &mut self,
        _floats: &mut Floats,
        _last_margin_bottom: Au,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_inline_block_width(containing_block);

        self.assign_padding();
        self.assign_border_width();
        self.assign_margin();
        // self.calculate_block_position(last_margin_bottom, containing_block);

        self.layout_block_children(viewport);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. https://www.w3.org/TR/CSS2/visudet.html#inlineblock-width
    fn calculate_inline_block_width(&mut self, _containing_block: Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        // TODO: Implement calculating shrink-to-fit width
        let auto = Value::Keyword("auto".to_string());
        let width = style.value("width").unwrap_or(auto.clone());

        if width == auto {
            // TODO
            panic!("calculating shrink-to-fit width is unsupported.");
        }

        self.dimensions.content.width = Au::from_f64_px(width.to_px().unwrap());
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

    pub fn assign_padding(&mut self) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.padding.left = Au::from_f64_px(
            style
                .lookup("padding-left", "padding", &zero)
                .to_px()
                .unwrap(),
        );
        d.padding.right = Au::from_f64_px(
            style
                .lookup("padding-right", "padding", &zero)
                .to_px()
                .unwrap(),
        );

        d.padding.top = Au::from_f64_px(
            style
                .lookup("padding-top", "padding", &zero)
                .to_px()
                .unwrap(),
        );
        d.padding.bottom = Au::from_f64_px(
            style
                .lookup("padding-bottom", "padding", &zero)
                .to_px()
                .unwrap(),
        );
    }

    pub fn assign_margin(&mut self) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.margin.left = Au::from_f64_px(
            style
                .lookup("margin-left", "margin", &zero)
                .to_px()
                .unwrap(),
        );
        d.margin.right = Au::from_f64_px(
            style
                .lookup("margin-right", "margin", &zero)
                .to_px()
                .unwrap(),
        );

        d.margin.top =
            Au::from_f64_px(style.lookup("margin-top", "margin", &zero).to_px().unwrap());
        d.margin.bottom = Au::from_f64_px(
            style
                .lookup("margin-bottom", "margin", &zero)
                .to_px()
                .unwrap(),
        );
    }

    pub fn assign_border_width(&mut self) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.border.left = Au::from_f64_px(
            style
                .lookup("border-left-width", "border-width", &zero)
                .to_px()
                .unwrap(),
        );
        d.border.right = Au::from_f64_px(
            style
                .lookup("border-width-right", "border-width", &zero)
                .to_px()
                .unwrap(),
        );

        d.border.top = Au::from_f64_px(
            style
                .lookup("border-width-top", "border-width", &zero)
                .to_px()
                .unwrap(),
        );
        d.border.bottom = Au::from_f64_px(
            style
                .lookup("border-width-bottom", "border-width", &zero)
                .to_px()
                .unwrap(),
        );
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
        try!(writeln!(f, "{:?}", self.dimensions));
        for child in &self.children {
            try!(write!(f, "{}", child));
        }
        Ok(())
    }
}
