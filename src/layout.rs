use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::{LayoutType, NodeType};
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Floats {
    pub float_list: FloatList,
    pub ceiling: Au,
    pub offset: EdgeSizes,
}
pub type FloatList = Vec<Float>;
#[derive(Clone, Debug)]
pub struct Float {
    pub dimensions: Dimensions,
    pub float_type: style::FloatType,
}
impl Float {
    pub fn new(dimensions: Dimensions, float_type: style::FloatType) -> Float {
        Float {
            dimensions: dimensions,
            float_type: float_type,
        }
    }
}
impl Floats {
    pub fn new() -> Floats {
        Floats {
            float_list: vec![],
            ceiling: Au(0),
            offset: EdgeSizes {
                top: Au(0),
                bottom: Au(0),
                left: Au(0),
                right: Au(0),
            },
        }
    }
    pub fn is_present(&self) -> bool {
        !self.float_list.is_empty()
    }
    pub fn translate(&mut self, delta: EdgeSizes) {
        self.offset.left += delta.left;
        self.offset.right += delta.right;
        // Need?
        // self.offset.top += delta.top;
        // self.offset.bottom += delta.bottom;
    }
    pub fn add_float(&mut self, float: Float) {
        self.float_list.push(float)
    }
    pub fn available_area(&mut self, max_width: Au, y: Au) -> Rect {
        let y = y + self.ceiling;
        let mut left = Au(0);
        let mut right = Au(0);
        for float in self.float_list.iter().rev() {
            let margin_box = float.dimensions.margin_box();
            match float.float_type {
                style::FloatType::Left => {
                    if left > Au(0) || (margin_box.y <= y && y <= margin_box.y + margin_box.height)
                    {
                        left += margin_box.width;
                    }
                }
                style::FloatType::Right => {
                    if right > Au(0) || (margin_box.y <= y && y <= margin_box.y + margin_box.height)
                    {
                        right += margin_box.width;
                    }
                }
                _ => unreachable!(),
            }
        }

        if left != Au(0) {
            left = Au::from_f64_px((left.to_f64_px() - self.offset.left.to_f64_px()).abs());
        }
        if right != Au(0) {
            right = Au::from_f64_px((right.to_f64_px() - self.offset.right.to_f64_px()).abs());
        }
        println!("{} {}", left.to_f64_px(), right.to_f64_px());

        Rect {
            x: left,
            y: Au(0),
            width: max_width - left - right,
            height: Au(0),
        }
    }
    pub fn left_width(&mut self) -> Au {
        self.float_list.iter().fold(Au(0), |acc, float| {
            acc + match float.float_type {
                style::FloatType::Left => float.dimensions.margin_box().width,
                _ => Au(0),
            }
        }) - self.offset.left
    }
    pub fn right_width(&mut self) -> Au {
        self.float_list.iter().fold(Au(0), |acc, float| {
            acc + match float.float_type {
                style::FloatType::Right => float.dimensions.margin_box().width,
                _ => Au(0),
            }
        }) - self.offset.right
    }
}

#[derive(Clone, Debug)]
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
    root_box.layout(Au(0), containing_block, saved_block, viewport);
    root_box
}

use std::cell::RefCell;

thread_local!(
    pub static IMG_CACHE: RefCell<HashMap<String, gdk_pixbuf::Pixbuf>> = {
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
                c.entry(image_url.clone())
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
    for child in &style_node.children {
        match (child.display(), child.float()) {
            (Display::Block, style::FloatType::None) => {
                root.children.push(build_layout_tree(child))
            }
            (Display::Inline, style::FloatType::None)
            | (Display::InlineBlock, style::FloatType::None) => root.get_inline_container()
                .children
                .push(build_layout_tree(child)),
            (_, style::FloatType::Left) | (_, style::FloatType::Right) => {
                root.children.push(build_layout_tree(child))
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
        last_margin_bottom: Au,
        containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        match self.box_type {
            BoxType::BlockNode => {
                self.layout_block(last_margin_bottom, containing_block, saved_block, viewport)
            }
            BoxType::InlineBlockNode => self.layout_inline_block(
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::AnonymousBlock => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = containing_block.content.height;

                let mut linemaker = LineMaker::new(self.children.clone(), self.floats.clone());
                linemaker.run(containing_block.content.width);
                linemaker.end_of_lines();
                linemaker.assign_position(containing_block.content.width);

                self.children = linemaker.new_boxes;
                self.dimensions.content.width = containing_block.content.width;
                self.dimensions.content.height = linemaker.cur_height;
            }
            BoxType::Float => {
                // TODO: Implement correctly ASAP!
                // Replaced Inline Element (<img>)
                let (width, height) = match &self.info {
                    &LayoutInfo::Image(ref pixbuf) => (
                        Au::from_f64_px(pixbuf.get_width() as f64),
                        Au::from_f64_px(pixbuf.get_height() as f64),
                    ),
                    _ => unimplemented!(),
                };
                self.dimensions.content.width = width;
                self.dimensions.content.height = height;
                self.dimensions.content.x = match self.style.unwrap().float() {
                    style::FloatType::Left => self.floats.left_width(),
                    style::FloatType::Right => {
                        containing_block.content.width - self.floats.right_width()
                            - self.dimensions.content.width
                    }
                    _ => unreachable!(),
                };
                self.dimensions.content.y = containing_block.content.height;
                self.z_index = 100000; //??
            }
            BoxType::InlineNode | BoxType::TextNode(_) => unreachable!(),
        }
    }

    /// Lay out a block-level element and its descendants.
    fn layout_block(
        &mut self,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        self.calculate_block_position(last_margin_bottom, containing_block);

        self.layout_block_children(viewport);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        let auto = Value::Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = sum([
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ].iter()
            .map(|v| v.to_px().unwrap_or(0.0)));

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.content.width.to_f64_px() {
            if margin_left == auto {
                margin_left = Value::Length(0.0, Unit::Px);
            }
            if margin_right == auto {
                margin_right = Value::Length(0.0, Unit::Px);
            }
        }

        // Adjust used values so that the above sum equals `containing_block.width`.
        // Each arm of the `match` should increase the total width by exactly `underflow`,
        // and afterward all values should be absolute lengths in px.
        let underflow = containing_block.content.width - Au::from_f64_px(total);

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are overconstrained, calculate margin_right.
            (false, false, false) => {
                margin_right = Value::Length(
                    margin_right.to_px().unwrap() + underflow.to_f64_px(),
                    Unit::Px,
                );
            }

            // If exactly one size is auto, its used value follows from the equality.
            (false, false, true) => {
                margin_right = Value::Length(underflow.to_f64_px(), Unit::Px);
            }
            (false, true, false) => {
                margin_left = Value::Length(underflow.to_f64_px(), Unit::Px);
            }

            // If width is set to auto, any other auto values become 0.
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Value::Length(0.0, Unit::Px);
                }
                if margin_right == auto {
                    margin_right = Value::Length(0.0, Unit::Px);
                }

                if underflow.to_f64_px() >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Value::Length(underflow.to_f64_px(), Unit::Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = Value::Length(0.0, Unit::Px);
                    margin_right = Value::Length(
                        margin_right.to_px().unwrap() + underflow.to_f64_px(),
                        Unit::Px,
                    );
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Value::Length(underflow.to_f64_px() / 2.0, Unit::Px);
                margin_right = Value::Length(underflow.to_f64_px() / 2.0, Unit::Px);
            }
        }

        let d = &mut self.dimensions;
        if let Some(width) = width.to_px() {
            d.content.width = Au::from_f64_px(width)
        }

        if let Some(padding_left) = padding_left.to_px() {
            d.padding.left = Au::from_f64_px(padding_left)
        }
        if let Some(padding_right) = padding_right.to_px() {
            d.padding.right = Au::from_f64_px(padding_right)
        }

        if let Some(border_left) = border_left.to_px() {
            d.border.left = Au::from_f64_px(border_left)
        }
        if let Some(border_right) = border_right.to_px() {
            d.border.right = Au::from_f64_px(border_right)
        }

        if let Some(margin_left) = margin_left.to_px() {
            d.margin.left = Au::from_f64_px(margin_left)
        }
        if let Some(margin_right) = margin_right.to_px() {
            d.margin.right = Au::from_f64_px(margin_right)
        }

        if self.floats.is_present() {
            self.floats.translate(d.left_right_offset());
        }
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block
    /// Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    fn calculate_block_position(&mut self, last_margin_bottom: Au, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top =
            Au::from_f64_px(style.lookup("margin-top", "margin", &zero).to_px().unwrap());
        d.margin.bottom = Au::from_f64_px(
            style
                .lookup("margin-bottom", "margin", &zero)
                .to_px()
                .unwrap(),
        );

        // Margin collapse
        // TODO: Is this implementation correct?
        if last_margin_bottom >= d.margin.top {
            d.margin.top = Au(0);
        } else {
            d.margin.top = d.margin.top - last_margin_bottom;
        }

        d.border.top = Au::from_f64_px(
            style
                .lookup("border-top-width", "border-width", &zero)
                .to_px()
                .unwrap(),
        );
        d.border.bottom = Au::from_f64_px(
            style
                .lookup("border-bottom-width", "border-width", &zero)
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

        self.z_index = style.lookup("z-index", "z-index", &zero).to_num() as i32;

        d.content.x = d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the block's children within its content area.
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self, viewport: Dimensions) {
        let d = &mut self.dimensions;
        let mut last_margin_bottom = Au(0);
        for child in &mut self.children {
            child.floats = {
                let mut floats = self.floats.clone();
                if !floats.float_list.is_empty() {
                    floats.ceiling = vec![floats.ceiling, d.content.height]
                        .into_iter()
                        .fold(Au(0), |x, y| x.max(y));
                }
                floats
            };

            child.layout(last_margin_bottom, *d, *d, viewport);

            match child.box_type {
                BoxType::Float => {
                    self.floats
                        .add_float(Float::new(child.dimensions, child.style.unwrap().float()));
                    child.dimensions.content.width = Au(0);
                    child.dimensions.content.height = Au(0);
                }
                _ => {}
            }

            last_margin_bottom = child.dimensions.margin.bottom;
            // Increment the height so each child is laid out below the previous one.
            d.content.height += child.dimensions.margin_box().height;
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(Value::Length(h, Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = Au::from_f64_px(h);
        }
    }

    /// Lay out a inline-block-level element and its descendants.
    fn layout_inline_block(
        &mut self,
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
    pub fn left_right_offset(self) -> EdgeSizes {
        EdgeSizes {
            top: Au(0),
            bottom: Au(0),
            left: self.left_offset(),
            right: self.right_offset(),
        }
    }
}

fn sum<I>(iter: I) -> f64
where
    I: Iterator<Item = f64>,
{
    iter.fold(0., |a, b| a + b)
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
