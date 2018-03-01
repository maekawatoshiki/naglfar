use cairo;
use pango;
use pangocairo;

use window::FONT_DESC;
use std::cell::RefCell;
use pango::LayoutExt;

thread_local!(pub static PANGO_LAYOUT: RefCell<pango::Layout> = {
    let surface = cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap();
    let ctx = pangocairo::functions::create_context(&cairo::Context::new(&surface)).unwrap();
    let layout = pango::Layout::new(&ctx);
    RefCell::new(layout)
});

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub size: f64,
    pub weight: FontWeight,
    pub slant: FontSlant,
}

#[derive(Clone, Copy, Debug)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Clone, Copy, Debug)]
pub enum FontSlant {
    Normal,
    Italic,
}

impl Font {
    pub fn new(size: f64, weight: FontWeight, slant: FontSlant) -> Font {
        Font {
            size: size,
            weight: weight,
            slant: slant,
        }
    }

    pub fn new_empty() -> Font {
        Font {
            size: 0.0,
            weight: FontWeight::Normal,
            slant: FontSlant::Normal,
        }
    }

    pub fn text_width(&self, text: &str) -> f64 {
        FONT_DESC.with(|font_desc| {
            let mut font_desc = font_desc.borrow_mut();
            font_desc.set_size(pango::units_from_double(px_to_pt(self.size)));
            font_desc.set_style(self.slant.to_pango_font_slant());
            font_desc.set_weight(self.weight.to_pango_font_weight());
            PANGO_LAYOUT.with(|layout| {
                let layout = layout.borrow_mut();
                layout.set_text(text);
                layout.set_font_description(Some(&*font_desc));
                pango::units_to_double(layout.get_size().0)
            })
        })
    }

    pub fn compute_max_chars(&self, s: &str, max_width: f64) -> usize {
        // TODO: Inefficient!
        // TODO: This code doesn't allow other than alphabets.
        let mut buf = "".to_string();
        let mut last_splittable_pos = s.len();
        for (i, c) in s.chars().enumerate() {
            buf.push(c);

            if c.is_whitespace() {
                last_splittable_pos = i;
            }

            let text_width = self.text_width(buf.as_str());
            if text_width > max_width {
                return last_splittable_pos + 1; // '1' means whitespace
            }
        }
        s.len()
    }
}

// TODO: any other better way?
fn px_to_pt(f: f64) -> f64 {
    f * 0.752
}
