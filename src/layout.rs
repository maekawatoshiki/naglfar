use style::{Display, StyledNode};
use css::{Unit, Value};
use dom::NodeType;
use std::default::Default;
use std::fmt;
use cairo::Context;
// use render::get_str_width;

// CSS box model. All sizes are in px.
// TODO: Support units other than px

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

// A node in the layout tree.
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

impl<'a> LayoutBox<'a> {
    pub fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    pub fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BoxType::BlockNode(node) | BoxType::InlineNode(node) => node,
            BoxType::AnonymousBlock => panic!("Anonymous block box has no style node"),
        }
    }
}

pub static DEFAULT_FONT_SIZE: f64 = 20.0;

// Transform a style tree into a layout tree.
pub fn layout_tree<'a>(
    node: &'a StyledNode<'a>,
    ctx: &Context,
    mut containing_block: Dimensions,
) -> LayoutBox<'a> {
    // The layout algorithm expects the container height to start at 0.
    // TODO: Save the initial containing block height, for calculating percent heights.
    containing_block.content.height = 0.0;

    let mut root_box = build_layout_tree(node, ctx);
    root_box.layout(ctx, containing_block);
    root_box
}

// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>, ctx: &Context) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(style_node),
        Display::Inline => BoxType::InlineNode(style_node),
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
    // Lay out a box and its descendants.
    fn layout(&mut self, ctx: &Context, containing_block: Dimensions) {
        match self.box_type {
            BoxType::BlockNode(_) => self.layout_block(ctx, containing_block),
            BoxType::InlineNode(_) => self.layout_inline(ctx, containing_block),
            BoxType::AnonymousBlock => {
                let mut containing_block = containing_block;
                containing_block.content.width = 0.0;
                for child in &mut self.children {
                    child.layout(ctx, containing_block);
                    containing_block.content.width += child.dimensions.margin_box().width;
                    self.dimensions.content.height = vec![
                        self.dimensions.content.height,
                        child.dimensions.margin_box().height,
                    ].into_iter()
                        .fold(0.0 / 0.0, f64::max);
                }
            }
        }
    }

    // Lay out a block-level element and its descendants.
    fn layout_block(&mut self, ctx: &Context, containing_block: Dimensions) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        self.calculate_block_position(containing_block);

        self.layout_block_children(ctx);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    // Lay out a inline-level element and its descendants.
    fn layout_inline(&mut self, ctx: &Context, containing_block: Dimensions) {
        self.calculate_inline_position(containing_block);

        self.layout_inline_children(ctx);

        // If the node is a text node, the text's width and height become
        // the node's width and height.
        match self.get_style_node().node.data {
            NodeType::Element(_) => {}
            NodeType::Text(ref body) => {
                ctx.set_font_size(DEFAULT_FONT_SIZE);
                let (width, descent) = {
                    let font_info = ctx.get_scaled_font();
                    let text_extents = font_info.text_extents(body.as_str());
                    (
                        text_extents.x_advance + text_extents.x_bearing,
                        font_info.extents().descent,
                    )
                };
                self.dimensions.content.width = width;
                self.dimensions.content.height = DEFAULT_FONT_SIZE + descent;
            }
        }
    }

    // https://www.w3.org/TR/CSS2/visudet.html#inline-replaced-height
    fn calculate_inline_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // TODO: Do follow specifications
        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();
        d.margin.left = style.lookup("margin-left", "margin", &zero).to_px();
        d.margin.right = style.lookup("margin-right", "margin", &zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();
        d.border.left = style
            .lookup("border-left-width", "border-width", &zero)
            .to_px();
        d.border.right = style
            .lookup("border-right-width", "border-width", &zero)
            .to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.width + containing_block.content.x + d.margin.left
            + d.border.left + d.padding.left;

        d.content.y = containing_block.content.height + containing_block.content.y + d.margin.top
            + d.border.top + d.padding.top;
    }

    // Lay out the inline's children within its content area.
    // Sets `self.dimensions.height` to the total content height.
    fn layout_inline_children(&mut self, ctx: &Context) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(ctx, *d);
            d.content.width += child.dimensions.margin_box().width; // TODO
        }
        d.content.height = DEFAULT_FONT_SIZE;
    }

    // Calculate the width of a block-level non-replaced element in normal flow.
    // ref. http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    // Sets the horizontal margin/padding/border dimensions, and the `width`.
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
            .map(|v| v.to_px()));

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.content.width {
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
        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are overconstrained, calculate margin_right.
            (false, false, false) => {
                margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
            }

            // If exactly one size is auto, its used value follows from the equality.
            (false, false, true) => {
                margin_right = Value::Length(underflow, Unit::Px);
            }
            (false, true, false) => {
                margin_left = Value::Length(underflow, Unit::Px);
            }

            // If width is set to auto, any other auto values become 0.
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Value::Length(0.0, Unit::Px);
                }
                if margin_right == auto {
                    margin_right = Value::Length(0.0, Unit::Px);
                }

                if underflow >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Value::Length(underflow, Unit::Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = Value::Length(0.0, Unit::Px);
                    margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Value::Length(underflow / 2.0, Unit::Px);
                margin_right = Value::Length(underflow / 2.0, Unit::Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    // Finish calculating the block's edge sizes, and position it within its containing block.
    // http://www.w3.org/TR/CSS2/visudet.html#normal-block
    // Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + containing_block.content.y + d.margin.top
            + d.border.top + d.padding.top;
    }

    // Lay out the block's children within its content area.
    // Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self, ctx: &Context) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(ctx, *d);
            // Increment the height so each child is laid out below the previous one.
            d.content.height += child.dimensions.margin_box().height;
        }
    }

    // Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(Value::Length(h, Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        } else {
            // When a block contains text. TODO: Is this correct?
            // https://www.w3.org/TR/2011/REC-CSS2-20110607/visudet.html#line-height
            let line_height = DEFAULT_FONT_SIZE * 1.2;
            self.dimensions.content.y -= (line_height - DEFAULT_FONT_SIZE) / 2.0;
            self.dimensions.content.height += (line_height - DEFAULT_FONT_SIZE) / 2.0;
        }
    }

    // Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            BoxType::InlineNode(_) | BoxType::AnonymousBlock => self,
            BoxType::BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(BoxType::AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
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
