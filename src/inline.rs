use css::Value;
use dom::NodeType;
use font::Font;
use layout::{BoxType, Dimensions, LayoutBox, LayoutInfo, Text};
use float::Floats;

use std::ops::Range;
use std::collections::{HashMap, VecDeque};
use std::cmp::max;

use gdk_pixbuf::PixbufExt;
use gdk_pixbuf;

use app_units::Au;

#[derive(Clone, Debug)]
pub struct Line {
    pub range: Range<usize>, // Range of LayoutBox(es) that represent(s) this line.
    pub metrics: LineMetrics,
    pub width: Au,
}

#[derive(Clone, Debug, Copy)]
pub struct LineMetrics {
    pub above_baseline: Au,
    pub under_baseline: Au,
}

impl LineMetrics {
    pub fn new(above_baseline: Au, under_baseline: Au) -> LineMetrics {
        LineMetrics {
            above_baseline: above_baseline,
            under_baseline: under_baseline,
        }
    }
    pub fn reset(&mut self) {
        self.above_baseline = Au(0);
        self.under_baseline = Au(0);
    }
    pub fn calculate_line_height(&self) -> Au {
        self.above_baseline + self.under_baseline
    }
}

#[derive(Clone, Debug)]
pub struct LineMaker {
    pub pending: Line,
    pub work_list: VecDeque<LayoutBox>,
    pub new_boxes: Vec<LayoutBox>,
    pub floats: Floats,
    pub lines: Vec<Line>,
    pub start: usize,
    pub end: usize,
    pub cur_width: Au,
    pub cur_height: Au,
    pub cur_metrics: LineMetrics,
}

impl LineMaker {
    pub fn new(boxes: Vec<LayoutBox>, floats: Floats) -> LineMaker {
        LineMaker {
            pending: Line {
                range: 0..0,
                metrics: LineMetrics::new(Au(0), Au(0)),
                width: Au(0),
            },
            work_list: VecDeque::from(boxes),
            new_boxes: vec![],
            floats: floats,
            lines: vec![],
            start: 0,
            end: 0,
            cur_width: Au(0),
            cur_height: Au(0),
            cur_metrics: LineMetrics::new(Au(0), Au(0)),
        }
    }

    pub fn run(&mut self, max_width: Au, containing_block: Dimensions) {
        while let Some(layoutbox) = self.work_list.pop_front() {
            if let BoxType::TextNode(ref text_info) = layoutbox.box_type {
                self.pending.range = text_info.range.clone()
            }

            let mut max_width_considered_float = self.floats
                .available_area(max_width, self.cur_height, Au(1))
                .width;

            match layoutbox.box_type {
                BoxType::TextNode(_) => while self.pending.range.len() != 0 {
                    self.run_on_text_node(layoutbox.clone(), max_width_considered_float);
                    max_width_considered_float = self.floats
                        .available_area(max_width, self.cur_height, Au(1))
                        .width;
                },
                BoxType::InlineBlockNode => {
                    self.run_on_inline_block_node(layoutbox, max_width_considered_float)
                }
                BoxType::InlineNode => {
                    self.run_on_inline_node(layoutbox, max_width_considered_float, containing_block)
                }
                _ => unimplemented!(),
            }
        }
    }

    pub fn calculate_width(&self) -> Au {
        let mut max_width = Au(0);
        for line in &self.lines {
            max_width = max(max_width, line.width);
        }
        max_width
    }

    pub fn flush_cur_line(&mut self) {
        // Push remainings to `lines`.
        self.lines.push(Line {
            range: self.start..self.end,
            metrics: self.cur_metrics,
            width: self.new_boxes[self.start..self.end]
                .iter()
                .fold(Au(0), |acc, lbox| acc + lbox.dimensions.margin_box().width),
        });
        self.cur_height += self.cur_metrics.calculate_line_height();
        self.start = self.end;
    }

    pub fn end_of_lines(&mut self) {
        self.flush_cur_line()
    }

    pub fn assign_position(&mut self, max_width: Au) {
        self.cur_height = Au(0);

        for line in &self.lines {
            self.cur_width = Au(0);

            for new_box in &mut self.new_boxes[line.range.clone()] {
                let (left_floats_width, max_width_considered_float) = {
                    let available_area =
                        self.floats
                            .available_area(max_width, self.cur_height, Au(1)); // magic number '1': Anything is ok if Au(x) > 0.
                    (available_area.x, available_area.width)
                };
                // TODO: Refine
                let text_align = new_box.property.text_align();
                let init_width = match text_align {
                    Value::Keyword(ref k) => match k.as_str() {
                        "center" => (max_width_considered_float - line.width) / 2,
                        "right" => max_width_considered_float - line.width,
                        "left" | _ => Au(0),
                    },
                    _ => Au(0),
                } + left_floats_width;

                new_box.dimensions.content.x = init_width + self.cur_width
                    + new_box.dimensions.padding.left
                    + new_box.dimensions.border.left
                    + new_box.dimensions.margin.left;

                // TODO: Refine
                let ascent = new_box.content_inline_ascent();
                new_box.dimensions.content.y =
                    self.cur_height + (line.metrics.above_baseline - ascent);

                self.cur_width += new_box.dimensions.margin_box().width;
            }
            self.cur_height += line.metrics.calculate_line_height();
        }
    }

    fn run_on_inline_node(
        &mut self,
        mut layoutbox: LayoutBox,
        max_width: Au,
        containing_block: Dimensions,
    ) {
        fn layout_text(
            mut layoutbox: LayoutBox,
            linemaker: &mut LineMaker,
            max_width: Au,
            containing_block: Dimensions,
        ) {
            linemaker.work_list = VecDeque::from(layoutbox.children.clone());
            layoutbox.children.clear();

            layoutbox.assign_padding();
            layoutbox.assign_border_width();

            let start = linemaker.end;

            linemaker.cur_width +=
                layoutbox.dimensions.padding.left + layoutbox.dimensions.border.left;
            linemaker.run(
                max_width
                    - (layoutbox.dimensions.padding.right + layoutbox.dimensions.border.right),
                containing_block,
            );
            linemaker.cur_width +=
                layoutbox.dimensions.padding.right + layoutbox.dimensions.border.right;

            let end = linemaker.end;

            let new_boxes_len = linemaker.new_boxes[start..end].len();
            let line_is_broken = linemaker.lines.len() > 0;

            for (i, new_box) in &mut linemaker.new_boxes[start..end].iter_mut().enumerate() {
                let mut layoutbox = layoutbox.clone();
                layoutbox.children.push(new_box.clone());

                macro_rules! f {
                    ($dst:expr, $src:expr, $name:ident) => {
                        $dst.dimensions.$name.top    = max($dst.dimensions.$name.top,    $src.dimensions.$name.top);
                        $dst.dimensions.$name.bottom = max($dst.dimensions.$name.bottom, $src.dimensions.$name.bottom);
                        $dst.dimensions.$name.left   = max($dst.dimensions.$name.left,   $src.dimensions.$name.left);
                        $dst.dimensions.$name.right  = max($dst.dimensions.$name.right,  $src.dimensions.$name.right);
                    };
                }

                // TODO: Is margin also needed?
                f!(layoutbox, new_box, padding);
                f!(layoutbox, new_box, border);

                if new_boxes_len > 1 && line_is_broken {
                    // TODO: Makes no sense!
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
        }
        // Non-replaced inline elements(like <span>)
        match layoutbox.info {
            LayoutInfo::Generic | LayoutInfo::Anker => {
                let mut linemaker = self.clone();

                layout_text(layoutbox, &mut linemaker, max_width, containing_block);

                self.new_boxes = linemaker.new_boxes;
                self.lines = linemaker.lines;
                self.start = linemaker.start;
                self.end = linemaker.end;
                self.cur_width = linemaker.cur_width;
                self.cur_height = linemaker.cur_height;
                self.cur_metrics = linemaker.cur_metrics;
            }
            LayoutInfo::Image(_) => {
                // Replaced Inline Element (<img>)
                let width;
                let height;
                layoutbox.layout_inline(&mut self.floats, containing_block);
                width = layoutbox.dimensions.border_box().width;
                height = layoutbox.dimensions.border_box().height;

                if self.cur_width + width > max_width {
                    self.flush_cur_line();
                    self.end += 1;

                    self.cur_width = width;
                    self.cur_metrics.above_baseline = height;
                } else {
                    self.end += 1;
                    self.cur_width += width;
                    self.cur_metrics.above_baseline = max(self.cur_metrics.above_baseline, height);
                }

                self.new_boxes.push(layoutbox);
            }
            LayoutInfo::Button(_, _) => {
                let btn_text = text(&layoutbox);
                use gtk::Button;
                use gtk::BinExt;
                use gtk::WidgetExt;
                use window::BUTTONS;
                // println!("d {:?}", d);

                let button = match &mut layoutbox.info {
                    &mut LayoutInfo::Button(ref mut btn, ref id) => {
                        let button = BUTTONS.with(|b| {
                            b.borrow_mut()
                                .entry(*id)
                                .or_insert_with(|| Button::new_with_label(btn_text.as_str()))
                                .clone()
                        });
                        *btn = Some(button.clone());
                        button
                    }
                    _ => unreachable!(),
                };
                use glib::prelude::*; // or `use gtk::prelude::*;`
                use gtk;
                let label = button
                    .get_child()
                    .unwrap()
                    .downcast::<gtk::Label>()
                    .unwrap();
                use pango;

                let mut linemaker = self.clone();
                layout_text(
                    layoutbox.clone(),
                    &mut linemaker,
                    max_width,
                    containing_block,
                );

                let font = get_font(&linemaker);
                use css::px2pt;
                label.set_markup(
                    format!(
                        "<span size='{}'>{}</span>",
                        pango::units_from_double(px2pt(font.size.to_f64_px())),
                        btn_text
                    ).as_str(),
                );
                use gtk::LabelExt;
                let button_height = button.get_allocated_height();
                button.set_valign(gtk::Align::Baseline);
                let width = Au::from_f64_px(label.get_allocated_width() as f64 + 10.0);

                let mut d = Au::from_f64_px(button_height as f64) - font.size;
                println!("height: {} {:?}", button_height, d);

                layoutbox.dimensions.content.width = width;
                layoutbox.dimensions.content.height = Au::from_f64_px(button_height as f64);

                layoutbox.children.clear();

                if self.cur_width + width > max_width {
                    self.flush_cur_line();
                    self.end += 1;

                    self.cur_width = width;
                } else {
                    self.end += 1;
                    self.cur_width += width;
                }
                self.cur_metrics.above_baseline = max(
                    // Au(0),
                    font.get_ascent_descent().0 + d / 2,
                    linemaker.cur_metrics.above_baseline,
                );
                self.cur_metrics.under_baseline = max(
                    // Au(0),
                    font.get_ascent_descent().1 + d / 2,
                    self.cur_metrics.under_baseline,
                );

                self.new_boxes.push(layoutbox);

                // Get the font found first
                fn get_font(linemaker: &LineMaker) -> Font {
                    fn font(b: &LayoutBox) -> Font {
                        if let BoxType::TextNode(Text { ref font, .. }) = b.box_type {
                            font.clone()
                        } else {
                            for child in &b.children {
                                return font(child);
                            }
                            panic!()
                        }
                    }
                    font(linemaker.new_boxes.last().unwrap())
                }
                fn text(b: &LayoutBox) -> String {
                    if let NodeType::Text(ref text) = b.node.data {
                        text.clone()
                    } else {
                        let mut t = "".to_string();
                        for child in &b.children {
                            t += text(&child).as_str();
                        }
                        t
                    }
                }
            }
            _ => {}
        }
    }

    fn run_on_inline_block_node(&mut self, mut layoutbox: LayoutBox, max_width: Au) {
        let mut containing_block: Dimensions = ::std::default::Default::default();
        containing_block.content.width = max_width - self.cur_width;
        layoutbox.layout(
            &mut self.floats,
            Au(0),
            containing_block,
            containing_block,
            containing_block,
        );

        let box_width = layoutbox.dimensions.margin_box().width;

        if self.cur_width + box_width > max_width {
            self.flush_cur_line();
            self.end += 1;

            self.cur_width = box_width;
            self.cur_metrics.above_baseline = max(
                self.cur_metrics.above_baseline,
                layoutbox.dimensions.margin_box().height,
            );

            self.new_boxes.push(layoutbox);
        } else {
            self.end += 1;
            self.cur_width += box_width;
            self.cur_metrics.above_baseline = max(
                self.cur_metrics.above_baseline,
                layoutbox.dimensions.margin_box().height,
            );

            self.new_boxes.push(layoutbox);
        }
    }

    fn run_on_text_node(&mut self, mut layoutbox: LayoutBox, max_width: Au) {
        let text = if let NodeType::Text(ref text) = layoutbox.node.data {
            &text[self.pending.range.clone()]
        } else {
            return;
        };

        let font_size = layoutbox.property.font_size();
        let line_height = layoutbox.property.line_height();
        let font_weight = layoutbox.property.font_weight();
        let font_slant = layoutbox.property.font_style();

        let my_font = Font::new(font_size, font_weight, font_slant);
        let text_width = Au::from_f64_px(my_font.text_width(text));
        let (ascent, descent) = my_font.get_ascent_descent();

        let mut new_layoutbox = layoutbox.clone();

        self.end += 1;

        self.cur_metrics.above_baseline = max(
            self.cur_metrics.above_baseline,
            ascent + (line_height - (ascent + descent)) / 2,
        );
        self.cur_metrics.under_baseline = max(
            self.cur_metrics.under_baseline,
            (line_height - (ascent + descent)) / 2 + descent,
        );

        if self.cur_width + text_width > max_width {
            let remaining_width = max_width - self.cur_width; // Is this correc?
            let max_chars = my_font.compute_max_chars(text, remaining_width.to_f64_px());

            new_layoutbox.dimensions.content.width =
                Au::from_f64_px(my_font.text_width(&text[0..max_chars]));
            new_layoutbox.dimensions.content.height = ascent + descent;

            new_layoutbox.set_text_info(
                Font {
                    size: font_size,
                    weight: font_weight,
                    slant: font_slant,
                },
                self.pending.range.start..self.pending.range.start + max_chars,
            );
            self.new_boxes.push(new_layoutbox.clone());

            self.pending.range = self.pending.range.start + max_chars..self.pending.range.end;

            self.flush_cur_line();

            self.cur_width = Au(0);
            self.cur_metrics.reset();
        } else {
            new_layoutbox.dimensions.content.width = text_width;
            new_layoutbox.dimensions.content.height = ascent + descent;

            new_layoutbox.set_text_info(
                Font {
                    size: font_size,
                    weight: font_weight,
                    slant: font_slant,
                },
                self.pending.range.start..text.len() + self.pending.range.start,
            );
            self.new_boxes.push(new_layoutbox.clone());

            self.pending.range = 0..0;

            self.cur_width += text_width;
        }
    }
}

impl LayoutBox {
    /// Lay out a inline-level element and its descendants.
    pub fn layout_inline(&mut self, _floats: &mut Floats, containing_block: Dimensions) {
        match self.info {
            LayoutInfo::Image(_) => {
                self.calculate_replaced_inline_width_height(containing_block);

                self.assign_padding();
                self.assign_border_width();
                self.assign_margin();
            }
            _ => unimplemented!(),
        }
    }

    /// Calculate the width of a inline-level replaced(<img>) element in normal flow.
    pub fn calculate_replaced_inline_width_height(&mut self, containing_block: Dimensions) {
        // Replaced Inline Element (<img>)
        let (width, height) = match &mut self.info {
            &mut LayoutInfo::Image(ref mut pixbuf) => {
                get_image(&self.node, pixbuf, containing_block)
            }
            _ => unimplemented!(),
        };

        self.dimensions.content.width = width;
        self.dimensions.content.height = height;
    }
}

impl LayoutBox {
    fn get_first_text_node(&self) -> Option<&LayoutBox> {
        match self.box_type {
            BoxType::TextNode(_) => Some(self),
            _ => {
                for child in &self.children {
                    if let Some(node) = child.get_first_text_node() {
                        return Some(node);
                    }
                }
                None
            }
        }
    }

    pub fn content_inline_ascent(&mut self) -> Au {
        let height = self.dimensions.content.height;
        match self.get_first_text_node() {
            Some(node) => match node.box_type {
                BoxType::TextNode(Text { font, .. }) => font.get_ascent_descent().0,
                _ => unreachable!(),
            },
            None => height,
        }
    }
}

// TODO: Implement correctly
impl LayoutBox {
    /// Lay out a inline-block-level element and its descendants.
    pub fn layout_inline_block(
        &mut self,
        _floats: &mut Floats,
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
    pub fn calculate_inline_block_width(&mut self, _containing_block: Dimensions) {
        // `width` has initial value `auto`.
        // TODO: Implement calculating shrink-to-fit width
        let auto = Value::Keyword("auto".to_string());
        let width = &self.property.value("width").unwrap_or(vec![auto.clone()])[0];

        if width == &auto {
            // TODO
            panic!("calculating shrink-to-fit width is unsupported.");
        }

        self.dimensions.content.width = Au::from_f64_px(width.to_px().unwrap());
    }
}
use dom::Node;
pub fn get_image(
    node: &Node,
    pixbuf: &mut Option<gdk_pixbuf::Pixbuf>,
    containing_block: Dimensions,
) -> (Au, Au) {
    let cb_width = containing_block.content.width.to_f64_px();
    let cb_height = containing_block.content.height.to_f64_px();

    let pixbuf = match pixbuf {
        &mut Some(ref pixbuf) => pixbuf.clone(),
        &mut None => {
            *pixbuf = Some(get_pixbuf(node));
            pixbuf.clone().unwrap()
        }
    };

    let specified_width_px = node.attr("width")
        .and_then(|w| w.maybe_percent_to_px(cb_width));
    // The same as above
    let specified_height_px = node.attr("height")
        .and_then(|h| h.maybe_percent_to_px(cb_height));

    match (specified_width_px, specified_height_px) {
        (Some(width), Some(height)) => (Au::from_f64_px(width), Au::from_f64_px(height)),
        (Some(width), None) => (
            Au::from_f64_px(width),
            Au::from_f64_px(width * (pixbuf.get_height() as f64 / pixbuf.get_width() as f64)),
        ),
        (None, Some(height)) => (
            Au::from_f64_px(height * (pixbuf.get_width() as f64 / pixbuf.get_height() as f64)),
            Au::from_f64_px(height),
        ),
        (None, None) => (
            Au::from_f64_px(pixbuf.get_width() as f64),
            Au::from_f64_px(pixbuf.get_height() as f64),
        ),
    }
}

use std::cell::RefCell;

type ImageKey = String; // URL

thread_local!(
    static IMG_CACHE: RefCell<HashMap<ImageKey, gdk_pixbuf::Pixbuf>> = {
        RefCell::new(HashMap::new())
    };
);

use interface::download;
pub fn get_pixbuf(node: &Node) -> gdk_pixbuf::Pixbuf {
    IMG_CACHE.with(|c| {
        let image_url = node.image_url().unwrap();
        c.borrow_mut()
            .entry(image_url.clone())
            .or_insert_with(|| {
                let (cache_name, _) = download(image_url.as_str());
                gdk_pixbuf::Pixbuf::new_from_file(cache_name.as_str()).unwrap()
            })
            .clone()
    })
}
