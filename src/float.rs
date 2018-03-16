use layout::{EdgeSizes, Rect};
use style;

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
        // Need?
        // self.offset.top += delta.top;
        // self.offset.bottom += delta.bottom;
    }

    pub fn add_float(&mut self, float: Float) {
        self.float_list.push(float)
    }

    pub fn available_area(&mut self, max_width: Au, ceiling: Au) -> Rect {
        let ceiling = ceiling + self.ceiling;
        let mut left = Au(0);
        let mut right = Au(0);

        for float in self.float_list.iter().rev() {
            match float.float_type {
                style::FloatType::Left => {
                    if left > Au(0)
                        || (float.rect.y <= ceiling && ceiling <= float.rect.y + float.rect.height)
                    {
                        left += float.rect.width;
                    }
                }
                style::FloatType::Right => {
                    if right > Au(0)
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
