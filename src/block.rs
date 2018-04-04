use css::{Unit, Value};
use float::Floats;
use layout::{BoxType, Dimensions, LayoutBox};

use std::cmp::max;

use app_units::Au;

impl<'a> LayoutBox<'a> {
    /// Lay out a block-level element and its descendants.
    pub fn layout_block(
        &mut self,
        floats: &mut Floats,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        self.floats = floats.clone();

        let margin = self.get_margin();
        let padding = self.get_padding();
        let border = self.get_border_width();
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(
            containing_block,
            margin.clone(),
            padding.clone(),
            border.clone(),
        );

        self.calculate_block_position(
            last_margin_bottom,
            containing_block,
            margin,
            padding,
            border,
        );

        if self.floats.is_present() {
            self.floats.translate(self.dimensions.offset());
        }

        self.layout_block_children(viewport);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    pub fn calculate_block_width(
        &mut self,
        containing_block: Dimensions,
        margin: (Value, Value, Value, Value),
        padding: (Value, Value, Value, Value),
        border: (Value, Value, Value, Value),
    ) {
        let style = self.get_style_node();
        let cb_width = containing_block.content.width.to_f64_px();

        // `width` has initial value `auto`.
        let auto = Value::Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(vec![auto.clone()])[0].clone();

        let mut margin_left = margin.3;
        let mut margin_right = margin.1;

        let border_left = border.3;
        let border_right = border.1;

        let padding_left = padding.3;
        let padding_right = padding.1;

        let total = sum([
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ].iter()
            .map(|v| v.maybe_percent_to_px(cb_width).unwrap_or(0.0)));

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
                    margin_right.maybe_percent_to_px(cb_width).unwrap() + underflow.to_f64_px(),
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
                        margin_right.maybe_percent_to_px(cb_width).unwrap() + underflow.to_f64_px(),
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
        if let Some(width) = width.maybe_percent_to_px(cb_width) {
            d.content.width = Au::from_f64_px(width)
        }

        if let Some(padding_left) = padding_left.maybe_percent_to_px(cb_width) {
            d.padding.left = Au::from_f64_px(padding_left)
        }
        if let Some(padding_right) = padding_right.maybe_percent_to_px(cb_width) {
            d.padding.right = Au::from_f64_px(padding_right)
        }

        if let Some(border_left) = border_left.maybe_percent_to_px(cb_width) {
            d.border.left = Au::from_f64_px(border_left)
        }
        if let Some(border_right) = border_right.maybe_percent_to_px(cb_width) {
            d.border.right = Au::from_f64_px(border_right)
        }

        if let Some(margin_left) = margin_left.maybe_percent_to_px(cb_width) {
            d.margin.left = Au::from_f64_px(margin_left)
        }
        if let Some(margin_right) = margin_right.maybe_percent_to_px(cb_width) {
            d.margin.right = Au::from_f64_px(margin_right)
        }
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block
    /// Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    pub fn calculate_block_position(
        &mut self,
        last_margin_bottom: Au,
        containing_block: Dimensions,
        margin: (Value, Value, Value, Value),
        padding: (Value, Value, Value, Value),
        border: (Value, Value, Value, Value),
    ) {
        let style = self.get_style_node();
        let cb_width = containing_block.content.width.to_f64_px();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        d.margin.top = Au::from_f64_px(margin.0.maybe_percent_to_px(cb_width).unwrap_or(0f64));
        d.margin.bottom = Au::from_f64_px(margin.2.maybe_percent_to_px(cb_width).unwrap_or(0f64));

        // Margin collapse
        // TODO: Is this implementation correct?
        if last_margin_bottom >= d.margin.top {
            d.margin.top = Au(0);
        } else {
            d.margin.top = d.margin.top - last_margin_bottom;
        }

        d.border.top = Au::from_f64_px(border.0.maybe_percent_to_px(cb_width).unwrap());
        d.border.bottom = Au::from_f64_px(border.2.maybe_percent_to_px(cb_width).unwrap());

        d.padding.top = Au::from_f64_px(padding.0.maybe_percent_to_px(cb_width).unwrap());
        d.padding.bottom = Au::from_f64_px(padding.2.maybe_percent_to_px(cb_width).unwrap());

        self.z_index = style.lookup("z-index", "z-index", &vec![zero])[0]
            .clone()
            .to_num() as i32;

        d.content.x = d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the block's children within its content area.
    /// Sets `self.dimensions.height` to the total content height.
    pub fn layout_block_children(&mut self, viewport: Dimensions) {
        let d = &mut self.dimensions;
        let mut last_margin_bottom = Au(0);
        let mut floats = &mut self.floats;

        // TODO: Consider a better way to position children.
        for child in &mut self.children {
            if let Some(style) = child.style {
                if let Some(clear) = style.clear() {
                    let clearance = floats.clearance(clear);
                    d.content.height += clearance;
                }
            }

            if floats.is_present() {
                floats.ceiling = max(floats.ceiling, d.content.height);
            }

            child.layout(&mut floats, last_margin_bottom, *d, *d, viewport);

            if child.box_type != BoxType::Float {
                last_margin_bottom = child.dimensions.margin.bottom;
                // Increment the height so each child is laid out below the previous one.
                d.content.height += child.dimensions.margin_box().height;
            }
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    pub fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(val) = self.get_style_node().value("height") {
            if let Value::Length(h, Unit::Px) = val[0] {
                self.dimensions.content.height = Au::from_f64_px(h);
            }
        }
    }
}

fn sum<I>(iter: I) -> f64
where
    I: Iterator<Item = f64>,
{
    iter.fold(0., |a, b| a + b)
}
