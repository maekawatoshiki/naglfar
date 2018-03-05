use css::{Unit, Value};
use dom::NodeType;
use std::ops::Range;
use font::Font;
use std::collections::VecDeque;
use layout::{BoxType, Dimensions, LayoutBox, LayoutInfo, DEFAULT_FONT_SIZE};

use app_units::Au;

#[derive(Clone, Debug)]
pub struct Line {
    pub range: Range<usize>, // layoutbox
    pub metrics: LineMetrics,
    pub width: f64,
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
                width: 0.0,
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

    pub fn run(&mut self, max_width: f64) {
        while let Some(layoutbox) = self.work_list.pop_front() {
            if let BoxType::TextNode(ref text_info) = layoutbox.box_type {
                self.pending.range = text_info.range.clone()
            }

            match layoutbox.box_type {
                BoxType::TextNode(_) => while self.pending.range.len() != 0 {
                    self.run_on_text_node(layoutbox.clone(), max_width)
                },
                BoxType::InlineBlockNode => self.run_on_inline_block_node(layoutbox, max_width),
                BoxType::InlineNode => self.run_on_inline_node(layoutbox, max_width),
                _ => unimplemented!(),
            }
        }
    }

    pub fn new_line(&mut self) {
        // Push remainings to `lines`.
        self.lines.push(Line {
            range: self.start..self.end,
            metrics: self.cur_metrics,
            width: self.new_boxes[self.start..self.end]
                .iter()
                .fold(0.0, |acc, lbox| {
                    acc + lbox.dimensions.margin_box().width.to_f64_px()
                }),
        });
        self.start = self.end;
    }
    pub fn end_of_lines(&mut self) {
        self.new_line()
    }

    pub fn assign_position(&mut self, max_width: f64) {
        self.cur_height = 0.0;

        for line in &self.lines {
            self.cur_width = 0.0;
            for new_box in &mut self.new_boxes[line.range.clone()] {
                // TODO: Refine
                let text_align = new_box
                    .get_style_node()
                    .value_with_default("text-align", &Value::Keyword("left".to_string()));
                let init_width = match text_align {
                    Value::Keyword(ref k) => match k.to_lowercase().as_str() {
                        "center" => (max_width - line.width) / 2.0,
                        "right" => max_width - line.width,
                        "left" | _ => 0.0,
                    },
                    _ => 0.0,
                };

                new_box.dimensions.content.x = Au::from_f64_px(init_width)
                    + Au::from_f64_px(self.cur_width)
                    + new_box.dimensions.padding.left
                    + new_box.dimensions.border.left
                    + new_box.dimensions.margin.left;
                // TODO: Refine
                new_box.dimensions.content.y = Au::from_f64_px(
                    self.cur_height
                        + (line.metrics.above_baseline
                            - new_box.dimensions.content.height.to_f64_px()),
                )
                    - (new_box.dimensions.padding.top + new_box.dimensions.margin.top
                        + new_box.dimensions.border.top);
                self.cur_width += new_box.dimensions.margin_box().width.to_f64_px();
            }
            self.cur_height += line.metrics.calculate_line_height();
        }
    }

    fn run_on_inline_node(&mut self, mut layoutbox: LayoutBox<'a>, max_width: f64) {
        // Non-replaced inline elements(like <span>)
        if layoutbox.style.unwrap().node.contains_text() {
            let mut linemaker = self.clone();
            linemaker.work_list = VecDeque::from(layoutbox.children.clone());
            layoutbox.children.clear();
            layoutbox.assign_padding();
            layoutbox.assign_border_width();
            let start = linemaker.end;
            linemaker.cur_width += layoutbox.dimensions.padding.left.to_f64_px()
                + layoutbox.dimensions.border.left.to_f64_px();
            linemaker.run(max_width);
            linemaker.cur_width += layoutbox.dimensions.padding.right.to_f64_px()
                + layoutbox.dimensions.border.right.to_f64_px();
            let end = linemaker.end;
            let new_boxes_len = linemaker.new_boxes[start..end].len();
            for (i, new_box) in &mut linemaker.new_boxes[start..end].iter_mut().enumerate() {
                let mut layoutbox = layoutbox.clone();
                layoutbox.children.push(new_box.clone());
                if new_boxes_len > 1 {
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
            self.new_boxes = linemaker.new_boxes;
            self.lines = linemaker.lines;
            self.start = linemaker.start;
            self.end = linemaker.end;
            self.cur_width = linemaker.cur_width;
            self.cur_metrics = linemaker.cur_metrics;
        } else {
            // Replaced Inline Element (<img>)
            let mut layoutbox = layoutbox;
            let (img_w, img_h) = if let &LayoutInfo::Image(ref pixbuf) = &layoutbox.info {
                (pixbuf.get_width() as f64, pixbuf.get_height() as f64)
            } else {
                panic!()
            };
            if self.cur_width + img_w > max_width {
                self.new_line();
                self.end += 1;

                self.cur_width = img_w;
                self.cur_metrics.above_baseline = vec![0.0, img_h as f64]
                    .into_iter()
                    .fold(0.0, |x, y| x.max(y));
                layoutbox.dimensions.content.width = Au::from_f64_px(img_w as f64);
                layoutbox.dimensions.content.height = Au::from_f64_px(img_h as f64);

                self.new_boxes.push(layoutbox);
            } else {
                self.end += 1;
                self.cur_width += img_w;
                self.cur_metrics.above_baseline =
                    vec![self.cur_metrics.above_baseline, img_h as f64]
                        .into_iter()
                        .fold(0.0, |x, y| x.max(y));
                layoutbox.dimensions.content.width = Au::from_f64_px(img_w as f64);
                layoutbox.dimensions.content.height = Au::from_f64_px(img_h as f64);
                self.new_boxes.push(layoutbox);
            }
        }
    }

    fn run_on_inline_block_node(&mut self, mut layoutbox: LayoutBox<'a>, max_width: f64) {
        let mut containing_block: Dimensions = ::std::default::Default::default();
        containing_block.content.width = Au::from_f64_px(max_width - self.cur_width);
        layoutbox.layout(Au(0), containing_block, containing_block, containing_block);

        let box_width = layoutbox.dimensions.margin_box().width.to_f64_px();

        if self.cur_width + box_width > max_width {
            self.new_line();
            self.end += 1;

            self.cur_width = box_width;
            self.cur_metrics.above_baseline = vec![
                self.cur_metrics.above_baseline,
                layoutbox.dimensions.margin_box().height.to_f64_px(),
            ].into_iter()
                .fold(0.0, |x, y| x.max(y));

            self.new_boxes.push(layoutbox);
        } else {
            self.end += 1;
            self.cur_width += box_width;
            self.cur_metrics.above_baseline = vec![
                self.cur_metrics.above_baseline,
                layoutbox.dimensions.margin_box().height.to_f64_px(),
            ].into_iter()
                .fold(0.0, |x, y| x.max(y));
            self.new_boxes.push(layoutbox);
        }
    }

    fn run_on_text_node(&mut self, layoutbox: LayoutBox<'a>, max_width: f64) {
        let style = layoutbox.style.unwrap();

        let text = if let NodeType::Text(ref text) = style.node.data {
            &text[self.pending.range.clone()]
        } else {
            return;
        };

        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        let font_size = style
            .lookup("font-size", "font-size", &default_font_size)
            .to_px()
            .unwrap();

        let line_height = font_size * 1.2; // TODO: magic number '1.2'

        let default_font_weight = Value::Keyword("normal".to_string());
        let font_weight = style
            .lookup("font-weight", "font-weight", &default_font_weight)
            .to_font_weight();

        let default_font_slant = Value::Keyword("normal".to_string());
        let font_slant = style
            .lookup("font-style", "font-style", &default_font_slant)
            .to_font_slant();

        // let text_width = font_info.text_extents(text).x_advance;
        let my_font = Font::new(font_size, font_weight, font_slant);
        let text_width = my_font.text_width(text);

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
            let max_chars = my_font.compute_max_chars(text, max_width - self.cur_width);

            new_layoutbox.dimensions.content.width =
                Au::from_f64_px(my_font.text_width(&text[0..max_chars]));
            new_layoutbox.dimensions.content.height = Au::from_f64_px(font_size);

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

            self.new_line();

            self.cur_width = 0.0;
            self.cur_metrics.reset();
        } else {
            new_layoutbox.dimensions.content.width = Au::from_f64_px(text_width);
            new_layoutbox.dimensions.content.height = Au::from_f64_px(font_size);

            new_layoutbox.set_text_info(
                Font {
                    size: font_size,
                    weight: font_weight,
                    slant: font_slant,
                },
                self.pending.range.start
                    ..self.pending.range.start + my_font.compute_max_chars(text, text_width),
            );
            self.new_boxes.push(new_layoutbox.clone());

            self.pending.range = 0..0;

            self.cur_width += text_width;
        }
    }
}
