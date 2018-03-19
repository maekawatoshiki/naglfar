use layout::{Dimensions, EdgeSizes, LayoutBox, LayoutInfo, Rect};
use style;
use css::{Unit, Value};

use gdk_pixbuf::PixbufExt;

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

    pub fn available_area(&mut self, max_width: Au, ceiling: Au) -> Rect {
        let ceiling = ceiling + self.ceiling + self.offset.top;
        let mut left = Au(0);
        let mut right = Au(0);

        for float in self.float_list.iter().rev() {
            match float.float_type {
                style::FloatType::Left => {
                    if (left > Au(0) && float.rect.y <= ceiling
                        && ceiling <= float.rect.y + float.rect.height)
                        || (float.rect.y <= ceiling && ceiling <= float.rect.y + float.rect.height)
                    {
                        left += float.rect.width;
                    }
                }
                style::FloatType::Right => {
                    if (right > Au(0) && float.rect.y <= ceiling
                        && ceiling <= float.rect.y + float.rect.height)
                        || (float.rect.y <= ceiling && ceiling <= float.rect.y + float.rect.height)
                    {
                        right += float.rect.width;
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

        Rect {
            x: left,
            y: Au(0),
            width: max_width - left - right,
            height: Au(0),
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

    pub fn left_width(&mut self) -> Au {
        self.float_list.iter().fold(Au(0), |acc, float| {
            acc + match float.float_type {
                style::FloatType::Left => float.rect.width,
                _ => Au(0),
            }
        }) - self.offset.left
    }
    pub fn right_width(&mut self) -> Au {
        self.float_list.iter().fold(Au(0), |acc, float| {
            acc + match float.float_type {
                style::FloatType::Right => float.rect.width,
                _ => Au(0),
            }
        }) - self.offset.right
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
                let pixbuf = match pixbuf {
                    &mut Some(ref pixbuf) => pixbuf.clone(),
                    _ => {
                        *pixbuf = Some(self.style.unwrap().get_pixbuf(containing_block));
                        pixbuf.clone().unwrap()
                    }
                };
                self.dimensions.content.width = Au::from_f64_px(pixbuf.get_width() as f64);
                self.dimensions.content.height = Au::from_f64_px(pixbuf.get_height() as f64);
            }
            LayoutInfo::Generic => {
                self.calculate_float_width(containing_block);
                self.assign_padding();
                self.assign_border_width();
                self.assign_margin();
                self.layout_block_children(viewport);
                self.calculate_block_height();
            }
        };

        let available_area = floats.available_area(containing_block.content.width, Au(0));
        self.dimensions.content.x = match self.style.unwrap().float() {
            style::FloatType::Left => self.dimensions.left_offset() + available_area.x,
            style::FloatType::Right => {
                available_area.width + available_area.x - self.dimensions.content.width
                    - self.dimensions.right_offset()
            }
            _ => unreachable!(),
        };
        self.dimensions.content.y = containing_block.content.height;

        floats.add_float(Float::new(
            self.dimensions.margin_box(),
            self.style.unwrap().float(),
        ));
    }
    /// Calculate the width of a float (non-replaced) element.
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    /// ref. https://www.w3.org/TR/2007/CR-CSS21-20070719/visudet.html#float-width
    // TODO: Implement correctly!
    pub fn calculate_float_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let cb_width = containing_block.content.width.to_f64_px();

        // `width` has initial value `auto`.
        let auto = Value::Keyword("auto".to_string());
        let width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let margin_left = style.lookup("margin-left", "margin", &zero);
        let margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

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
}
