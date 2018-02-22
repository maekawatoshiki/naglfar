use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::NodeType;
use std::default::Default;
use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;

use cairo::{Context, ScaledFont};
use cairo;

use app_units::Au;
// use render::get_str_width;

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

// A node in the layout tree.
#[derive(Clone, Debug)]
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub z_index: i32,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

#[derive(Clone, Debug)]
pub enum BoxType<'a> {
    BlockNode(StyledNode<'a>),
    InlineNode(StyledNode<'a>),
    TextNode(StyledNode<'a>, Text),
    AnonymousBlock(Texts),
}

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub size: f64,
    pub weight: FontWeight,
}

#[derive(Clone, Copy, Debug)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub font: Font,
    pub range: Range<usize>,
}

pub type Texts = Vec<Text>;

#[derive(Clone, Debug)]
pub struct Line {
    pub range: Range<usize>, // layoutbox
    pub metrics: LineMetrics,
}

#[derive(Clone, Debug, Copy)]
pub struct LineMetrics {
    pub above_baseline: f64,
    pub under_baseline: f64,
}

impl LineMetrics {
    pub fn new(above_baseline: f64, under_baseline: f64) -> LineMetrics {
        LineMetrics {
            above_baseline: above_baseline,
            under_baseline: under_baseline,
        }
    }
    pub fn reset(&mut self) {
        self.above_baseline = 0.0;
        self.under_baseline = 0.0;
    }
    pub fn calculate_line_height(&self) -> f64 {
        self.above_baseline + self.under_baseline
    }
}

#[derive(Clone, Debug)]
pub struct LineMaker<'a> {
    pub pending: Line,
    pub work_list: VecDeque<LayoutBox<'a>>,
    pub new_boxes: Vec<LayoutBox<'a>>,
    pub lines: Vec<Line>,
    pub start: usize,
    pub end: usize,
    pub cur_width: f64,
    pub cur_height: f64,
    pub cur_metrics: LineMetrics,
}

impl<'a> LineMaker<'a> {
    pub fn new(boxes: Vec<LayoutBox<'a>>) -> LineMaker {
        LineMaker {
            pending: Line {
                range: 0..0,
                metrics: LineMetrics::new(0.0, 0.0),
            },
            work_list: VecDeque::from(boxes),
            new_boxes: vec![],
            lines: vec![],
            start: 0,
            end: 0,
            cur_width: 0.0,
            cur_height: 0.0,
            cur_metrics: LineMetrics::new(0.0, 0.0),
        }
    }

    pub fn run(&mut self, ctx: &Context, max_width: f64) {
        while let Some(mut layoutbox) = self.work_list.pop_front() {
            if let BoxType::TextNode(_, ref text_info) = layoutbox.box_type {
                self.pending.range = text_info.range.clone()
            }

            match layoutbox.box_type {
                BoxType::TextNode(_, _) => while self.pending.range.len() != 0 {
                    self.run_text_node(layoutbox.clone(), ctx, max_width)
                },
                BoxType::InlineNode(_) => {
                    let mut linemaker = self.clone();
                    linemaker.work_list = VecDeque::from(layoutbox.children.clone());
                    layoutbox.children.clear();
                    layoutbox.assign_inline_padding();
                    layoutbox.assign_inline_border_width();
                    let start = linemaker.end;
                    linemaker.cur_width += layoutbox.dimensions.padding.left.to_f64_px()
                        + layoutbox.dimensions.border.left.to_f64_px();
                    linemaker.run(ctx, max_width);
                    linemaker.cur_width += layoutbox.dimensions.padding.right.to_f64_px()
                        + layoutbox.dimensions.border.right.to_f64_px();
                    let end = linemaker.end;
                    let new_boxes_len = linemaker.new_boxes[start..end].len();
                    for (i, new_box) in &mut linemaker.new_boxes[start..end].iter_mut().enumerate()
                    {
                        let mut layoutbox = layoutbox.clone();
                        layoutbox.children.push(new_box.clone());
                        if new_boxes_len > 1 {
                            if i == 0 {
                                layoutbox.dimensions.padding.right = Au(0);
                                layoutbox.dimensions.border.right = Au(0);
                            } else if i == new_boxes_len - 1 {
                                layoutbox.dimensions.padding.left = Au(0);
                                layoutbox.dimensions.border.left = Au(0);
                            } else {
                                layoutbox.dimensions.padding.left = Au(0);
                                layoutbox.dimensions.padding.right = Au(0);
                                layoutbox.dimensions.border.left = Au(0);
                                layoutbox.dimensions.border.right = Au(0);
                            }
                        }
                        layoutbox.dimensions.content.width = new_box.dimensions.content.width;
                        layoutbox.dimensions.content.height = new_box.dimensions.content.height;
                        *new_box = layoutbox;
                    }
                    self.new_boxes = linemaker.new_boxes;
                    self.lines = linemaker.lines;
                    self.start = linemaker.start;
                    self.end = linemaker.end;
                    self.cur_width = linemaker.cur_width;
                    self.cur_metrics = linemaker.cur_metrics;
                }
                _ => {}
            }
        }
    }
    fn end_of_lines(&mut self) {
        // push remainings to `lines`.
        self.lines.push(Line {
            range: self.start..self.end,
            metrics: self.cur_metrics,
        });
        self.start = self.end;
    }
    fn assign_position(&mut self) {
        self.cur_height = 0.0;

        for line in &self.lines {
            self.cur_width = 0.0;
            for new_box in &mut self.new_boxes[line.range.clone()] {
                new_box.dimensions.content.x = Au::from_f64_px(self.cur_width)
                    + new_box.dimensions.padding.left
                    + new_box.dimensions.border.left;
                // TODO: fix
                new_box.dimensions.content.y = Au::from_f64_px(
                    self.cur_height
                        + (line.metrics.above_baseline
                            - new_box.dimensions.content.height.to_f64_px()),
                );
                self.cur_width += new_box.dimensions.border_box().width.to_f64_px();
            }
            self.cur_height += line.metrics.calculate_line_height();
        }
    }
    fn run_text_node(&mut self, layoutbox: LayoutBox<'a>, ctx: &Context, max_width: f64) {
        let style = if let BoxType::TextNode(s, _) = layoutbox.box_type.clone() {
            s
        } else {
            return;
        };

        let text = if let NodeType::Text(ref text) = style.node.data {
            &text[self.pending.range.clone()]
        } else {
            return;
        };

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();

        let line_height = font_size * 1.2; // TODO: magic number '1.2'

        let default_font_weight = Value::Keyword("normal".to_string());
        let font_weight = style
            .lookup("font-weight", "font-weight", &default_font_weight)
            .to_font_weight();

        ctx.set_font_size(font_size);
        ctx.select_font_face(
            "",
            cairo::FontSlant::Normal,
            font_weight.to_cairo_font_weight(),
        );
        let font_info = ctx.get_scaled_font();
        let text_width = font_info.text_extents(text).x_advance;

        let mut new_layoutbox = layoutbox.clone();

        self.end += 1;

        self.cur_metrics.above_baseline = vec![
            self.cur_metrics.above_baseline,
            line_height - (line_height - font_size) / 2.0,
        ].into_iter()
            .fold(0.0, |x, y| x.max(y));
        self.cur_metrics.under_baseline = vec![
            self.cur_metrics.under_baseline,
            (line_height - font_size) / 2.0,
        ].into_iter()
            .fold(0.0, |x, y| x.max(y));

        if self.cur_width + text_width > max_width {
            self.lines.push(Line {
                range: self.start..self.end,
                metrics: self.cur_metrics,
            });

            self.start = self.end;

            let max_chars = compute_max_chars(text, max_width - self.cur_width, &font_info);

            new_layoutbox.dimensions.content.width =
                Au::from_f64_px(font_info.text_extents(&text[0..max_chars]).x_advance);
            new_layoutbox.dimensions.content.height = Au::from_f64_px(font_size);

            new_layoutbox.set_text_info(
                Font {
                    size: font_size,
                    weight: font_weight,
                },
                self.pending.range.start..self.pending.range.start + max_chars,
            );
            self.new_boxes.push(new_layoutbox.clone());

            self.pending.range = self.pending.range.start + max_chars..self.pending.range.end;

            self.cur_width = 0.0;
            self.cur_metrics.reset();
        } else {
            new_layoutbox.dimensions.content.width = Au::from_f64_px(text_width);
            new_layoutbox.dimensions.content.height = Au::from_f64_px(font_size);

            new_layoutbox.set_text_info(
                Font {
                    size: font_size,
                    weight: font_weight,
                },
                self.pending.range.start
                    ..self.pending.range.start + compute_max_chars(text, text_width, &font_info),
            );
            self.new_boxes.push(new_layoutbox.clone());

            self.pending.range = 0..0;

            self.cur_width += text_width;
        }
    }
}

fn compute_max_chars(s: &str, max_width: f64, font_info: &ScaledFont) -> usize {
    // TODO: Inefficient!
    // TODO: This code doesn't allow other than alphabets.
    let mut buf = "".to_string();
    let mut last_splittable_pos = s.len();
    for (i, c) in s.chars().enumerate() {
        buf.push(c);

        if c.is_whitespace() {
            last_splittable_pos = i;
        }

        let text_width = font_info.text_extents(buf.as_str()).x_advance;
        if text_width > max_width {
            return last_splittable_pos + 1; // '1' means whitespace
        }
    }
    s.len()
}

impl<'a> LayoutBox<'a> {
    pub fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            z_index: 0,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    pub fn get_style_node(&self) -> Option<&StyledNode<'a>> {
        match self.box_type {
            BoxType::BlockNode(ref node)
            | BoxType::InlineNode(ref node)
            | BoxType::TextNode(ref node, _) => Some(node),
            BoxType::AnonymousBlock(_) => None,
        }
    }

    pub fn set_text_info(&mut self, font: Font, range: Range<usize>) {
        if let BoxType::TextNode(_, ref mut r) = self.box_type {
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
    ctx: &Context,
    mut containing_block: Dimensions,
) -> LayoutBox<'a> {
    // Save the initial containing block height for calculating percent heights.
    let saved_block = containing_block.clone();
    let viewport = containing_block.clone();
    // The layout algorithm expects the container height to start at 0.
    containing_block.content.height = Au::from_f64_px(0.0);

    let mut root_box = build_layout_tree(node, ctx);
    root_box.layout(ctx, Au(0), containing_block, saved_block, viewport);
    root_box
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>, ctx: &Context) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(style_node.clone()),
        Display::Inline => match style_node.node.data {
            NodeType::Element(_) => BoxType::InlineNode(style_node.clone()),
            NodeType::Text(ref s) => BoxType::TextNode(
                style_node.clone(),
                Text {
                    font: Font {
                        size: 0.0,
                        weight: FontWeight::Normal,
                    },
                    range: 0..s.len(),
                },
            ),
        },
        Display::None => panic!("Root node has display: none."),
    });

    // Create the descendant boxes.
    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child, ctx)),
            Display::Inline => root.get_inline_container()
                .children
                .push(build_layout_tree(child, ctx)),
            Display::None => {} // Don't lay out nodes with `display: none;`
        }
    }
    root
}

impl<'a> LayoutBox<'a> {
    /// Lay out a box and its descendants.
    /// `saved_block` is used to know the maximum width/height of the box, calculate the percent
    /// width/height and so on.
    fn layout(
        &mut self,
        ctx: &Context,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        match self.box_type {
            BoxType::BlockNode(_) => self.layout_block(
                ctx,
                last_margin_bottom,
                containing_block,
                saved_block,
                viewport,
            ),
            BoxType::AnonymousBlock(ref mut _texts) => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = containing_block.content.height;

                let mut linemaker = LineMaker::new(self.children.clone());
                linemaker.run(ctx, containing_block.padding_box().width.to_f64_px());
                linemaker.end_of_lines();
                linemaker.assign_position();
                self.children = linemaker.new_boxes;
                self.dimensions.content.width = containing_block.content.width;
                self.dimensions.content.height = Au::from_f64_px(linemaker.cur_height);

                println!("{}", self.dimensions.content.height.to_f64_px());
            }
            BoxType::InlineNode(_) | BoxType::TextNode(_, _) => unreachable!(),
        }
    }

    /// Lay out a block-level element and its descendants.
    fn layout_block(
        &mut self,
        ctx: &Context,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        self.calculate_block_position(last_margin_bottom, containing_block);

        self.layout_block_children(ctx, viewport);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node().unwrap().clone();

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
            .map(|v| v.to_px()));

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
                margin_right =
                    Value::Length(margin_right.to_px() + underflow.to_f64_px(), Unit::Px);
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
                    margin_right =
                        Value::Length(margin_right.to_px() + underflow.to_f64_px(), Unit::Px);
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Value::Length(underflow.to_f64_px() / 2.0, Unit::Px);
                margin_right = Value::Length(underflow.to_f64_px() / 2.0, Unit::Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = Au::from_f64_px(width.to_px());

        d.padding.left = Au::from_f64_px(padding_left.to_px());
        d.padding.right = Au::from_f64_px(padding_right.to_px());

        d.border.left = Au::from_f64_px(border_left.to_px());
        d.border.right = Au::from_f64_px(border_right.to_px());

        d.margin.left = Au::from_f64_px(margin_left.to_px());
        d.margin.right = Au::from_f64_px(margin_right.to_px());
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block
    /// Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    fn calculate_block_position(&mut self, last_margin_bottom: Au, containing_block: Dimensions) {
        let style = self.get_style_node().unwrap().clone();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = Au::from_f64_px(style.lookup("margin-top", "margin", &zero).to_px());
        d.margin.bottom = Au::from_f64_px(style.lookup("margin-bottom", "margin", &zero).to_px());

        d.margin.top = Au::from_f64_px((last_margin_bottom - d.margin.top).to_f64_px().abs());

        d.border.top = Au::from_f64_px(
            style
                .lookup("border-top-width", "border-width", &zero)
                .to_px(),
        );
        d.border.bottom = Au::from_f64_px(
            style
                .lookup("border-bottom-width", "border-width", &zero)
                .to_px(),
        );

        d.padding.top = Au::from_f64_px(style.lookup("padding-top", "padding", &zero).to_px());
        d.padding.bottom =
            Au::from_f64_px(style.lookup("padding-bottom", "padding", &zero).to_px());

        self.z_index = style.lookup("z-index", "z-index", &zero).to_num() as i32;

        d.content.x = d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the block's children within its content area.
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self, ctx: &Context, viewport: Dimensions) {
        let d = &mut self.dimensions;
        let mut last_margin_bottom = Au(0);
        for child in &mut self.children {
            child.layout(ctx, last_margin_bottom, *d, *d, viewport);
            last_margin_bottom = child.dimensions.margin.bottom;
            // Increment the height so each child is laid out below the previous one.
            d.content.height += child.dimensions.margin_box().height;
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(Value::Length(h, Unit::Px)) = self.get_style_node().unwrap().value("height") {
            self.dimensions.content.height = Au::from_f64_px(h);
        }
    }

    /// Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            BoxType::InlineNode(_) | BoxType::AnonymousBlock(_) => self,
            BoxType::BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBlock(_),
                        ..
                    }) => {}
                    _ => self.children
                        .push(LayoutBox::new(BoxType::AnonymousBlock(Texts::new()))),
                }
                self.children.last_mut().unwrap()
            }
            BoxType::TextNode(_, _) => panic!(),
        }
    }

    fn assign_inline_padding(&mut self) {
        let style = self.get_style_node().unwrap().clone();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.padding.left = Au::from_f64_px(style.lookup("padding-left", "padding", &zero).to_px());
        d.padding.right = Au::from_f64_px(style.lookup("padding-right", "padding", &zero).to_px());

        d.padding.top = Au::from_f64_px(style.lookup("padding-top", "padding", &zero).to_px());
        d.padding.bottom =
            Au::from_f64_px(style.lookup("padding-bottom", "padding", &zero).to_px());
    }

    fn assign_inline_border_width(&mut self) {
        let style = self.get_style_node().unwrap().clone();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.border.left = Au::from_f64_px(
            style
                .lookup("border-left-width", "border-width", &zero)
                .to_px(),
        );
        d.border.right = Au::from_f64_px(
            style
                .lookup("border-width-right", "border-width", &zero)
                .to_px(),
        );

        d.border.top = Au::from_f64_px(
            style
                .lookup("border-width-top", "border-width", &zero)
                .to_px(),
        );
        d.border.bottom = Au::from_f64_px(
            style
                .lookup("border-width-bottom", "border-width", &zero)
                .to_px(),
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
}

impl Value {
    pub fn to_font_weight(&self) -> FontWeight {
        match self {
            &Value::Keyword(ref k) if k.as_str() == "normal" => FontWeight::Normal,
            &Value::Keyword(ref k) if k.as_str() == "bold" => FontWeight::Bold,
            _ => FontWeight::Normal,
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
