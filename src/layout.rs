use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::NodeType;
use std::default::Default;
use std::fmt;
use cairo::Context;
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
    pub rect: Rect,
    pub line: i32, // Starts with 0
    pub text: String,
    pub font: Font,
    pub line_height: f64,
}

pub type Texts = Vec<Text>;

#[derive(Clone, Debug)]
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    TextNode(&'a StyledNode<'a>),
    AnonymousBlock(Texts),
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

    pub fn get_style_node(&self) -> Option<&'a StyledNode<'a>> {
        match self.box_type {
            BoxType::BlockNode(node) | BoxType::InlineNode(node) | BoxType::TextNode(node) => {
                Some(node)
            }
            BoxType::AnonymousBlock(_) => None,
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
    root_box.calculate_inline_size(ctx);
    root_box.calculate_block_size(ctx, containing_block);
    root_box.calculate_position(
        ctx,
        &mut Texts::new(),
        containing_block,
        saved_block,
        viewport,
    );
    root_box
}

// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>, ctx: &Context) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(style_node),
        Display::Inline => match style_node.node.data {
            NodeType::Element(_) => BoxType::InlineNode(style_node),
            NodeType::Text(_) => BoxType::TextNode(style_node),
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
    fn calculate_inline_size(&mut self, ctx: &Context) {
        match self.box_type {
            BoxType::BlockNode(_) => for child in &mut self.children {
                child.calculate_inline_size(ctx);
            },
            BoxType::InlineNode(_) => self.do_calculate_inline_size(ctx),
            BoxType::TextNode(_) => self.do_calculate_text_size_inline(ctx),
            BoxType::AnonymousBlock(_) => for child in &mut self.children {
                child.calculate_inline_size(ctx);
            },
        }
    }
    fn do_calculate_inline_size(&mut self, ctx: &Context) {
        let style = self.get_style_node().unwrap();
        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();

        self.dimensions.content.height = Au::from_f64_px(font_size);

        for child in &mut self.children {
            child.calculate_inline_size(ctx);
            self.dimensions.content.width += child.dimensions.content.width;
        }
    }
    fn do_calculate_text_size_inline(&mut self, ctx: &Context) {
        let style = self.get_style_node().unwrap();

        let text = if let NodeType::Text(ref text) = style.node.data {
            text
        } else {
            return;
        };

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();
        let line_height = font_size * 1.2;

        let default_font_weight = Value::Keyword("normal".to_string());
        let font_weight = style
            .lookup("font-weight", "font-weight", &default_font_weight)
            .to_font_weight();

        // TODO: REFINE THIS!
        ctx.set_font_size(font_size);
        ctx.select_font_face(
            "",
            cairo::FontSlant::Normal,
            font_weight.to_cairo_font_weight(),
        );
        let font_info = ctx.get_scaled_font();
        let text_width = font_info.text_extents(text.as_str()).x_advance;

        self.dimensions.content.width = Au::from_f64_px(text_width);
        self.dimensions.content.height = Au::from_f64_px(font_size);
    }
    fn calculate_block_size(&mut self, ctx: &Context, mut containing_block: Dimensions) {
        match self.box_type {
            BoxType::BlockNode(_) => self.do_calculate_block_size(ctx, containing_block),
            BoxType::InlineNode(_) => {}
            BoxType::TextNode(_) => {}
            BoxType::AnonymousBlock(_) => {
                self.dimensions.content.width = containing_block.content.width;

                containing_block.content.width = Au(0);

                for child in &mut self.children {
                    let child_width = child.dimensions.content.width;

                    if self.dimensions.content.width < containing_block.content.width + child_width
                    {
                        match child.box_type {
                            BoxType::InlineNode(_) | BoxType::TextNode(_) => {
                                self.dimensions.content.height = vec![
                                    self.dimensions.content.height,
                                    Au::from_f64_px(
                                        ((child.dimensions.content.width
                                            / self.dimensions.content.width)
                                            as i32 as f64
                                            + 1.0)
                                            * child.dimensions.content.height.to_f64_px(),
                                    ),
                                ].into_iter()
                                    .fold(Au::from_f64_px(0.0), |x, y| if x < y { y } else { x });
                            }
                            _ => panic!(),
                        }
                        containing_block.content.width = Au(0);
                    } else {
                        containing_block.content.width += child_width;

                        // Inline and text elements are already calculated their size.
                        self.dimensions.content.height = vec![
                            self.dimensions.content.height,
                            child.dimensions.margin_box().height,
                        ].into_iter()
                            .fold(Au::from_f64_px(0.0), |x, y| if x < y { y } else { x });
                    }
                }
                self.dimensions.content.height =
                    Au::from_f64_px(self.dimensions.content.height.to_f64_px() * 1.2);
            }
        }
    }
    fn do_calculate_block_size(&mut self, ctx: &Context, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);

        for child in &mut self.children {
            child.calculate_block_size(ctx, self.dimensions);
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }

        self.calculate_block_height(ctx);
    }

    fn calculate_position(
        &mut self,
        ctx: &Context,
        texts: &mut Texts,
        mut containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        match self.box_type {
            BoxType::BlockNode(_) => {
                self.calculate_block_position(containing_block);
                containing_block.content.height = Au::from_f64_px(0.0);
                for child in &mut self.children {
                    child.calculate_position(ctx, texts, containing_block, saved_block, viewport);
                    containing_block.content.height += child.dimensions.margin_box().height;
                }
            }
            BoxType::InlineNode(_) => {
                self.calculate_inline_position(containing_block);

                // containing_block.content.width = Au(0);

                for child in &mut self.children {
                    child.calculate_position(ctx, texts, containing_block, saved_block, viewport);
                    containing_block.content.width += child.dimensions.margin_box().width;
                }
            }
            BoxType::TextNode(_) => {
                self.calculate_text_position(ctx, texts, containing_block, saved_block, viewport)
            }
            BoxType::AnonymousBlock(ref mut texts) => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = containing_block.content.height;

                containing_block.content.width = Au(0);
                containing_block.content.height = Au(0);

                // TODO
                let line_height = 19.2;
                let font_size = 16.0;

                let l = (line_height - font_size);

                let mut line = 0;

                for child in &mut self.children {
                    child.calculate_position(
                        ctx,
                        texts,
                        containing_block,
                        self.dimensions,
                        viewport,
                    );

                    let child_width = child.dimensions.margin_box().width;

                    if self.dimensions.content.width < containing_block.content.width + child_width
                    {
                        let style = child.get_style_node().unwrap();
                        match child.box_type {
                            BoxType::TextNode(_) => {
                                let text = if let NodeType::Text(ref text) = style.node.data {
                                    text
                                } else {
                                    panic!()
                                };

                                let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
                                let font_size = style
                                    .lookup("font-size", "font-size", &default_font_size)
                                    .to_px();
                                let line_height = font_size * 1.2;

                                let default_font_weight = Value::Keyword("normal".to_string());
                                let font_weight = style
                                    .lookup("font-weight", "font-weight", &default_font_weight)
                                    .to_font_weight();

                                // TODO: REFINE THIS!
                                ctx.set_font_size(font_size);
                                ctx.select_font_face(
                                    "",
                                    cairo::FontSlant::Normal,
                                    font_weight.to_cairo_font_weight(),
                                );
                                let font_info = ctx.get_scaled_font();
                                let font_width = font_info.extents().max_x_advance;

                                let mut tt = vec![];
                                let mut s = "".to_string();
                                let mut max =
                                    self.dimensions.content.width - containing_block.content.width;

                                for c in text.chars() {
                                    if font_info.text_extents(s.as_str()).x_advance + font_width
                                        > max.to_f64_px()
                                    {
                                        tt.push(s.clone());
                                        s.clear();
                                        max = self.dimensions.content.width;
                                    }
                                    s.push(c);
                                }
                                tt.push(s.clone());

                                texts.pop();

                                for (i, t) in tt.iter().enumerate() {
                                    texts.push(Text {
                                        rect: Rect {
                                            x: if i == 0 {
                                                containing_block.content.width
                                            } else {
                                                Au(0)
                                            },
                                            y: Au::from_f64_px(line_height - font_size) / 2
                                                + Au::from_f64_px(i as f64 * line_height),
                                            width: Au::from_f64_px(
                                                font_info.text_extents(t.as_str()).x_advance,
                                            ),
                                            height: Au::from_f64_px(font_size),
                                        },
                                        line: i as i32,
                                        font: Font {
                                            size: font_size,
                                            weight: font_weight,
                                        },
                                        text: t.clone(),
                                        line_height: line_height,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }

                    child.dimensions.content.y += Au::from_f64_px(l) / 2;

                    containing_block.content.width += child_width;
                }
            }
        }
    }
    fn calculate_text_position(
        &mut self,
        ctx: &Context,
        texts: &mut Texts,
        mut containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        let style = self.get_style_node().unwrap();

        let text = if let NodeType::Text(ref text) = style.node.data {
            text
        } else {
            return;
        };

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();
        let line_height = font_size * 1.2;

        let default_font_weight = Value::Keyword("normal".to_string());
        let font_weight = style
            .lookup("font-weight", "font-weight", &default_font_weight)
            .to_font_weight();

        // TODO: REFINE THIS!
        ctx.set_font_size(font_size);
        ctx.select_font_face(
            "",
            cairo::FontSlant::Normal,
            font_weight.to_cairo_font_weight(),
        );
        let font_info = ctx.get_scaled_font();
        let text_width = font_info.text_extents(text.as_str()).x_advance;
        let font_width = font_info.extents().max_x_advance;

        let max_width = saved_block.content.width;

        self.dimensions.content.x = Au::from_f64_px(0.0);
        self.dimensions.content.y = Au::from_f64_px(0.0);

        texts.push(Text {
            rect: {
                let mut d = self.dimensions.content;
                d.x = containing_block.content.width;
                d.y = Au::from_f64_px(line_height - font_size) / 2;
                d
            },
            line: 0,
            text: text.clone(),
            font: Font {
                size: font_size,
                weight: font_weight,
            },
            line_height: line_height,
        });
    }

    /// Lay out a box and its descendants.
    /// `saved_block` is used to know the maximum width/height of the box, calculate the percent
    /// width/height and so on.
    fn layout(
        &mut self,
        ctx: &Context,
        texts: &mut Texts,
        mut containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        match self.box_type {
            BoxType::BlockNode(_) => {
                self.layout_block(ctx, texts, containing_block, saved_block, viewport)
            }
            BoxType::InlineNode(_) => {
                self.layout_inline(ctx, texts, containing_block, saved_block, viewport)
            }
            BoxType::TextNode(_) => {
                self.layout_inline(ctx, texts, containing_block, saved_block, viewport)
            }
            BoxType::AnonymousBlock(_) => {
                self.dimensions.content.x = Au::from_f64_px(0.0);
                self.dimensions.content.y = Au::from_f64_px(0.0);

                containing_block.content.width = Au::from_f64_px(0.0);

                for child in &mut self.children {
                    child.layout(ctx, texts, containing_block, saved_block, viewport);

                    let child_width = child.dimensions.margin_box().width;

                    containing_block.content.width += child_width;
                    self.dimensions.content.width += child_width;
                    self.dimensions.content.height = vec![
                        self.dimensions.content.height,
                        child.dimensions.margin_box().height,
                    ].into_iter()
                        .fold(Au::from_f64_px(0.0), |x, y| if x < y { y } else { x });
                }
            }
        }
    }

    /// Lay out a block-level element and its descendants.
    fn layout_block(
        &mut self,
        ctx: &Context,
        texts: &mut Texts,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        self.calculate_block_position(containing_block);

        self.layout_block_children(ctx, texts, viewport);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height(ctx);
    }

    /// Lay out a inline-level element and its descendants.
    fn layout_inline(
        &mut self,
        ctx: &Context,
        texts: &mut Texts,
        containing_block: Dimensions,
        saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        self.calculate_inline_position(containing_block);

        self.layout_inline_children(ctx, texts, viewport);

        self.layout_text(ctx, texts, saved_block);

        // TODO: Is this correct?
        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = self.get_style_node()
            .unwrap()
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();
        let line_height = font_size * 1.2;
        self.dimensions.content.y += Au::from_f64_px(line_height - font_size) / 2;
    }

    /// Lay out a text.
    fn layout_text(&mut self, ctx: &Context, texts: &mut Texts, saved_block: Dimensions) {
        let style = self.get_style_node().unwrap();

        let text = if let NodeType::Text(ref text) = style.node.data {
            text
        } else {
            return;
        };

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();
        let line_height = font_size * 1.2;

        let default_font_weight = Value::Keyword("normal".to_string());
        let font_weight = style
            .lookup("font-weight", "font-weight", &default_font_weight)
            .to_font_weight();

        // TODO: REFINE THIS!
        ctx.set_font_size(font_size);
        ctx.select_font_face(
            "",
            cairo::FontSlant::Normal,
            font_weight.to_cairo_font_weight(),
        );
        let font_info = ctx.get_scaled_font();
        let text_width = font_info.text_extents(text.as_str()).x_advance;
        let font_width = font_info.extents().max_x_advance;

        let max_width = saved_block.content.width;

        // If line breaking is needed.
        if max_width.to_f64_px() > 0.0 && max_width.to_f64_px() < text_width {
            let mut line = "".to_string();
            let mut d = self.dimensions;

            for c in text.chars() {
                line.push(c);

                // TODO: It's inefficient to call `text_extents` every time.
                if max_width.to_f64_px()
                    - (d.content.x.to_f64_px() - saved_block.content.x.to_f64_px())
                    - font_width
                    < font_info.text_extents(line.as_str()).x_advance
                {
                    // texts.push(Text {
                    //     dimensions: d,
                    //     text: line.clone(),
                    //     font: Font {
                    //         size: font_size,
                    //         weight: font_weight,
                    //     },
                    // });
                    d.content.x = saved_block.content.x;
                    d.content.y += Au::from_f64_px(line_height);
                    line.clear();
                }
            }

            // texts.push(Text {
            //     dimensions: d,
            //     text: line.clone(),
            //     font: Font {
            //         size: font_size,
            //         weight: font_weight,
            //     },
            // });

            self.dimensions.content.width =
                max_width - (self.dimensions.content.x - saved_block.content.x);
            self.dimensions.content.height =
                Au::from_f64_px(line_height) * (text_width as i32 / max_width.to_px());
        } else {
            // texts.push(Text {
            //     dimensions: self.dimensions,
            //     text: text.clone(),
            //     font: Font {
            //         size: font_size,
            //         weight: font_weight,
            //     },
            // });
            self.dimensions.content.width = Au::from_f64_px(text_width);
            self.dimensions.content.height = Au::from_f64_px(line_height);
        };
    }

    /// Finish calculating the inline's edge sizes, and position it within its containing block.
    /// https://www.w3.org/TR/CSS2/visudet.html#inline-replaced-height
    fn calculate_inline_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node().unwrap();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // TODO: Follow specifications
        d.margin.left = Au::from_f64_px(style.lookup("margin-left", "margin", &zero).to_px());
        d.margin.right = Au::from_f64_px(style.lookup("margin-right", "margin", &zero).to_px());

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
        d.border.left = Au::from_f64_px(
            style
                .lookup("border-left-width", "border-width", &zero)
                .to_px(),
        );
        d.border.right = Au::from_f64_px(
            style
                .lookup("border-right-width", "border-width", &zero)
                .to_px(),
        );

        d.padding.top = Au::from_f64_px(style.lookup("padding-top", "padding", &zero).to_px());
        d.padding.bottom =
            Au::from_f64_px(style.lookup("padding-bottom", "padding", &zero).to_px());

        d.content.x =
            containing_block.content.width + d.margin.left + d.border.left + d.padding.left;

        d.content.y = containing_block.content.height + d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the inline's children within its content area.
    /// Sets `self.dimensions.width` to the total content width and
    /// sets `self.dimensions.height` to default font size(height).
    fn layout_inline_children(&mut self, ctx: &Context, texts: &mut Texts, viewport: Dimensions) {
        let style = self.get_style_node().unwrap();
        let d = &mut self.dimensions;

        for child in &mut self.children {
            // child.layout(ctx, &mut texts, *d, *d, viewport);
            d.content.width += child.dimensions.margin_box().width; // TODO
        }

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px();

        d.content.height = Au::from_f64_px(font_size);
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node().unwrap();

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
    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node().unwrap();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = Au::from_f64_px(style.lookup("margin-top", "margin", &zero).to_px());
        d.margin.bottom = Au::from_f64_px(style.lookup("margin-bottom", "margin", &zero).to_px());

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
    fn layout_block_children(&mut self, ctx: &Context, texts: &mut Texts, viewport: Dimensions) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(ctx, texts, *d, *d, viewport);
            // Increment the height so each child is laid out below the previous one.
            d.content.height += child.dimensions.margin_box().height;
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self, ctx: &Context) {
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
            BoxType::TextNode(_) => panic!(),
        }
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
    pub fn add_xy(self, x: Au, y: Au) -> Rect {
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
