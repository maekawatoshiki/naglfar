use layout::{BoxType, Dimensions, EdgeSizes, LayoutBox, LayoutInfo, Rect};
use inline::get_image;
use style;
use css::Value;

use std::cmp::{max, min};

use app_units::Au;

#[derive(Clone, Debug)]
pub struct Floats {
    pub float_list: FloatList,
    pub ceiling: Au,
    pub offset: EdgeSizes,
}

pub type FloatList = Vec<Float>;

#[derive(Clone, Debug)]
pub struct Float {
    pub rect: Rect,
    pub float_type: style::FloatType,
}

impl Float {
    pub fn new(rect: Rect, float_type: style::FloatType) -> Float {
        Float {
            rect: rect,
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
        self.offset.top += delta.top;
        self.offset.bottom += delta.bottom;
    }

    pub fn add_float(&mut self, float: Float) {
        self.float_list.push(float)
    }

    pub fn available_area(&mut self, max_width: Au, ceiling: Au, height: Au) -> Rect {
        let ceiling = ceiling + self.ceiling + self.offset.top;
        let mut left = Au(0);
        let mut right = Au(0);
        let mut l_ceiling = None;
        let mut r_ceiling = None;
        let mut l_height = None;
        let mut r_height = None;

        for float in self.float_list.iter() {
            match float.float_type {
                style::FloatType::Left
                    if float.rect.x + float.rect.width > left
                        && float.rect.y + float.rect.height > ceiling
                        && float.rect.y < ceiling + height =>
                {
                    left = float.rect.x + float.rect.width;
                    l_ceiling = Some(float.rect.y);
                    l_height = Some(float.rect.height);
                }
                style::FloatType::Right
                    if (max_width - float.rect.x) > right
                        && float.rect.y + float.rect.height > ceiling
                        && float.rect.y < ceiling + height =>
                {
                    right += float.rect.width;
                    r_ceiling = Some(float.rect.y);
                    r_height = Some(float.rect.height);
                }
                _ => {}
            }
        }

        if left != Au(0) {
            left = Au::from_f64_px((left.to_f64_px() - self.offset.left.to_f64_px()).abs());
        }
        if right != Au(0) {
            right = Au::from_f64_px((right.to_f64_px() - self.offset.right.to_f64_px()).abs());
        }

        let (ceiling, height) = match (l_ceiling, l_height, r_ceiling, r_height) {
            (Some(l_ceiling), Some(l_height), Some(r_ceiling), Some(r_height)) => (
                max(max(l_ceiling, ceiling), max(r_ceiling, ceiling)),
                min(max(l_height, height), max(r_height, height)),
            ),
            (None, None, Some(r_ceiling), Some(r_height)) => (max(ceiling, r_ceiling), r_height),
            (Some(l_ceiling), Some(l_height), None, None) => (max(ceiling, l_ceiling), l_height),
            (None, None, None, None) => {
                return Rect {
                    x: Au(0),
                    y: Au(0),
                    width: max_width,
                    height: Au(0),
                }
            }
            _ => unreachable!(),
        };

        Rect {
            x: left,
            y: ceiling,
            width: max_width - left - right,
            height: height,
        }
    }

    pub fn clearance(&mut self, clear_type: style::ClearType) -> Au {
        let mut clearance = Au(0);
        for float in &self.float_list {
            match (clear_type, float.float_type) {
                (style::ClearType::Left, style::FloatType::Left)
                | (style::ClearType::Right, style::FloatType::Right)
                | (style::ClearType::Both, _) => {
                    let b = self.offset.top + float.rect.y + float.rect.height;
                    clearance = ::std::cmp::max(clearance, b);
                }
                _ => {}
            }
        }
        clearance
    }
}

impl<'a> LayoutBox<'a> {
    pub fn layout_float(
        &mut self,
        floats: &mut Floats,
        _last_margin_bottom: Au,
        containing_block: Dimensions,
        _saved_block: Dimensions,
        viewport: Dimensions,
    ) {
        // TODO: Implement correctly ASAP!
        // Replaced Inline Element (<img>)
        match self.info {
            LayoutInfo::Image(ref mut pixbuf) => {
                let (width, height) = get_image(self.style.unwrap(), pixbuf, containing_block);
                self.dimensions.content.width = width;
                self.dimensions.content.height = height;
            }
            LayoutInfo::Generic => {
                self.assign_padding();
                self.assign_border_width();
                self.assign_margin();

                let width_not_specified = self.calculate_float_width(containing_block);

                if width_not_specified {
                    let children = self.children.clone();
                    let floats = self.floats.clone();
                    self.layout_float_children(viewport);

                    self.dimensions.content.width = Au(0);
                    for child in &self.children {
                        match self.box_type {
                            BoxType::BlockNode | BoxType::AnonymousBlock => {
                                self.dimensions.content.width = max(
                                    self.dimensions.content.width,
                                    child.dimensions.margin_box().width,
                                );
                            }
                            BoxType::Float => {
                                // Ignore whether the float is on left or on right
                                self.dimensions.content.width +=
                                    child.dimensions.margin_box().width;
                            }
                            _ => {}
                        }
                    }

                    // Recalculate the descendants' size and position.
                    // TODO: More efficient implementation needed
                    self.dimensions.content.height = Au(0);
                    self.floats = floats;
                    self.children = children;
                    self.layout_float_children(viewport);
                } else {
                    self.layout_float_children(viewport);
                }

                self.calculate_block_height();
            }
            _ => unimplemented!(),
        };

        self.calculate_float_position(floats, containing_block);

        floats.add_float(Float::new(
            self.dimensions.margin_box(),
            self.style.unwrap().float(),
        ));
    }

    pub fn layout_float_children(&mut self, viewport: Dimensions) {
        self.layout_block_children(viewport);
        // The height of float children in a float element is noticed.
        self.dimensions.content.height = max(
            self.dimensions.content.height,
            self.floats.clearance(style::ClearType::Both),
        );
    }

    pub fn calculate_float_position(&mut self, floats: &mut Floats, containing_block: Dimensions) {
        let mut float_height = Au(0);
        loop {
            let margin_box = self.dimensions.margin_box();
            let available_area = floats.available_area(
                containing_block.content.width,
                float_height,
                margin_box.height,
            );
            if margin_box.width <= available_area.width {
                self.dimensions.content.x = match self.style.unwrap().float() {
                    style::FloatType::Left => self.dimensions.left_offset() + available_area.x,
                    style::FloatType::Right => {
                        available_area.width + available_area.x - self.dimensions.content.width
                            - self.dimensions.right_offset()
                    }
                    _ => unreachable!(),
                };
                self.dimensions.content.y = containing_block.content.height + float_height;
                break;
            } else {
                if available_area.height == Au(0) {
                    // There is no available area.
                    break;
                }
                float_height += available_area.height;
            }
        }
    }

    /// Calculate the width of a float (non-replaced) element.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. https://www.w3.org/TR/2007/CR-CSS21-20070719/visudet.html#float-width
    // TODO: Implement correctly!
    pub fn calculate_float_width(&mut self, containing_block: Dimensions) -> bool {
        let style = self.get_style_node();
        let cb_width = containing_block.content.width.to_f64_px();

        // `width` has initial value `auto`.
        let auto = Value::Keyword("auto".to_string());
        let width = style.value("width").unwrap_or(vec![auto.clone()])[0].clone();

        let d = &mut self.dimensions;

        let mut width_not_specified = false;
        if width == auto {
            width_not_specified = true;
            d.content.width = containing_block.content.width;
        } else if let Some(width) = width.maybe_percent_to_px(cb_width) {
            d.content.width = Au::from_f64_px(width)
        }

        width_not_specified
    }
}
