use cairo;
use pango;
use pangocairo;

use css::px2pt;

use std::cell::RefCell;
use pango::{ContextExt, LayoutExt};

use app_units::Au;

thread_local!(
    pub static PANGO_LAYOUT: RefCell<pango::Layout> = {
        let surface = cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap();
        let ctx = pangocairo::functions::create_context(&cairo::Context::new(&surface)).unwrap();
        let layout = pango::Layout::new(&ctx);
        RefCell::new(layout)
    };
    pub static FONT_DESC: RefCell<pango::FontDescription> = {
        RefCell::new(pango::FontDescription::from_string("sans-serif normal 16"))
    }
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Font {
    pub size: Au,
    pub weight: FontWeight,
    pub slant: FontSlant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontSlant {
    Normal,
    Italic,
}

impl Font {
    pub fn new(size: Au, weight: FontWeight, slant: FontSlant) -> Font {
        FONT_DESC.with(|font_desc| {
            let mut font_desc = font_desc.borrow_mut();
            font_desc.set_size(pango::units_from_double(px2pt(size.to_f64_px())));
            font_desc.set_style(slant.to_pango_font_slant());
            font_desc.set_weight(weight.to_pango_font_weight());
            PANGO_LAYOUT.with(|layout| {
                layout.borrow_mut().set_font_description(Some(&*font_desc));
            })
        });

        Font {
            size: size,
            weight: weight,
            slant: slant,
        }
    }

    pub fn new_empty() -> Font {
        Font {
            size: Au(0),
            weight: FontWeight::Normal,
            slant: FontSlant::Normal,
        }
    }

    pub fn text_width(&self, text: &str) -> f64 {
        PANGO_LAYOUT.with(|layout| {
            let layout = layout.borrow_mut();
            layout.set_text(text);
            pango::units_to_double(layout.get_size().0)
        })
    }

    pub fn get_ascent_descent(&self) -> (Au, Au) {
        FONT_DESC.with(|font_desc| {
            let font_desc = font_desc.borrow();
            PANGO_LAYOUT.with(|layout| {
                let ctx = layout.borrow_mut().get_context().unwrap();
                let metrics =
                    ctx.get_metrics(Some(&*font_desc), Some(&pango::Language::from_string("")))
                        .unwrap();
                (
                    Au::from_f64_px(pango::units_to_double(metrics.get_ascent()) as f64),
                    Au::from_f64_px(pango::units_to_double(metrics.get_descent()) as f64),
                )
            })
        })
    }

    pub fn compute_max_chars_and_width(&self, s: &str, max_width: f64) -> (usize, f64) {
        if max_width < 0f64 {
            return (0, 0.0);
        }

        PANGO_LAYOUT.with(|layout| {
            let layout = layout.borrow_mut();
            // TODO: Inefficient implementation!
            let mut text_width = 0.0;
            let mut last_splittable_pos = None;
            let mut last_pos = 0;
            for (pos, c) in s.char_indices() {
                if c.is_whitespace() || c.is_ascii_punctuation() {
                    last_splittable_pos = Some(pos);
                }

                layout.set_text(c.to_string().as_str());
                let c_width = pango::units_to_double(layout.get_size().0);
                text_width += c_width;

                if text_width > max_width {
                    if let Some(pos) = last_splittable_pos {
                        return (pos + 1, text_width - c_width); // '1' means whitespace or punctuation.
                    } else {
                        if pos == 0 {
                            break;
                        }
                        if pos - last_pos > 1 {
                            // if c is multi-byte character
                            return (pos, text_width - c_width);
                        }
                    }
                }

                last_pos = pos;
            }

            (if s.is_empty() { 0 } else { 1 }, text_width)
        })
    }
}
