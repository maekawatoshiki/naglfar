use css::{Color, TextDecoration, Unit, Value, pt2px};
use font::{FontSlant, FontWeight};

use std::collections::HashMap;

use app_units::Au;

#[derive(Clone, Debug)]
pub struct Style(pub HashMap<String, Vec<Value>>);

impl Style {
    pub fn new() -> Style {
        Style(HashMap::new())
    }

    pub fn new_with(hashmap: HashMap<String, Vec<Value>>) -> Style {
        Style(hashmap)
    }
}

#[derive(PartialEq, Debug)]
pub enum Display {
    Inline,
    Block,
    InlineBlock,
    None,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum FloatType {
    Left,
    Right,
    None,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum ClearType {
    Left,
    Right,
    Both,
}

pub const DEFAULT_FONT_SIZE: f64 = 16.0f64;
pub const DEFAULT_LINE_HEIGHT_SCALE: f64 = 1.2f64;

impl Style {
    pub fn value(&self, name: &str) -> Option<Vec<Value>> {
        self.0.get(name).cloned()
    }

    pub fn value_with_default(&self, name: &str, default: &Vec<Value>) -> Vec<Value> {
        self.value(name).unwrap_or(default.clone())
    }

    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Vec<Value>) -> Vec<Value> {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    pub fn lookup_without_default(&self, name: &str, fallback_name: &str) -> Option<Vec<Value>> {
        self.value(name).or_else(|| self.value(fallback_name))
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "block" => Display::Block,
                    "inline-block" => Display::InlineBlock,
                    "none" => Display::None,
                    "inline" | _ => Display::Inline,
                },
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }

    pub fn float(&self) -> FloatType {
        match self.value("float") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "left" => FloatType::Left,
                    "right" => FloatType::Right,
                    "none" => FloatType::None,
                    _ => FloatType::None,
                },
                _ => FloatType::None,
            },
            _ => FloatType::None,
        }
    }

    pub fn clear(&self) -> Option<ClearType> {
        match self.value("clear") {
            Some(x) => match x[0] {
                Value::Keyword(ref s) => match &**s {
                    "left" => Some(ClearType::Left),
                    "right" => Some(ClearType::Right),
                    "both" => Some(ClearType::Both),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    pub fn padding(&self) -> (Value, Value, Value, Value) {
        // padding has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut padding_top = self.value("padding-top").and_then(|x| Some(x[0].clone()));
        let mut padding_bottom = self.value("padding-bottom")
            .and_then(|x| Some(x[0].clone()));
        let mut padding_left = self.value("padding-left").and_then(|x| Some(x[0].clone()));
        let mut padding_right = self.value("padding-right").and_then(|x| Some(x[0].clone()));

        if let Some(padding) = self.value("padding") {
            match padding.len() {
                1 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_bottom.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[0].clone());
                    padding_right.get_or_insert_with(|| padding[0].clone());
                }
                2 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_bottom.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[1].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                }
                3 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_left.get_or_insert_with(|| padding[1].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                    padding_bottom.get_or_insert_with(|| padding[2].clone());
                }
                4 => {
                    padding_top.get_or_insert_with(|| padding[0].clone());
                    padding_right.get_or_insert_with(|| padding[1].clone());
                    padding_bottom.get_or_insert_with(|| padding[2].clone());
                    padding_left.get_or_insert_with(|| padding[3].clone());
                }
                0 | _ => unreachable!(),
            }
        }

        padding_top.get_or_insert_with(|| zero.clone());
        padding_right.get_or_insert_with(|| zero.clone());
        padding_bottom.get_or_insert_with(|| zero.clone());
        padding_left.get_or_insert_with(|| zero.clone());

        (
            padding_top.unwrap(),
            padding_right.unwrap(),
            padding_bottom.unwrap(),
            padding_left.unwrap(),
        )
    }

    pub fn margin(&self) -> (Value, Value, Value, Value) {
        // margin has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_top = self.value("margin-top").and_then(|x| Some(x[0].clone()));
        let mut margin_bottom = self.value("margin-bottom").and_then(|x| Some(x[0].clone()));
        let mut margin_left = self.value("margin-left").and_then(|x| Some(x[0].clone()));
        let mut margin_right = self.value("margin-right").and_then(|x| Some(x[0].clone()));

        if let Some(margin) = self.value("margin") {
            match margin.len() {
                1 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_bottom.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[0].clone());
                    margin_right.get_or_insert_with(|| margin[0].clone());
                }
                2 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_bottom.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[1].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                }
                3 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_left.get_or_insert_with(|| margin[1].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                    margin_bottom.get_or_insert_with(|| margin[2].clone());
                }
                4 => {
                    margin_top.get_or_insert_with(|| margin[0].clone());
                    margin_right.get_or_insert_with(|| margin[1].clone());
                    margin_bottom.get_or_insert_with(|| margin[2].clone());
                    margin_left.get_or_insert_with(|| margin[3].clone());
                }
                0 | _ => unreachable!(),
            }
        }

        margin_top.get_or_insert_with(|| zero.clone());
        margin_right.get_or_insert_with(|| zero.clone());
        margin_bottom.get_or_insert_with(|| zero.clone());
        margin_left.get_or_insert_with(|| zero.clone());

        (
            margin_top.unwrap(),
            margin_right.unwrap(),
            margin_bottom.unwrap(),
            margin_left.unwrap(),
        )
    }

    pub fn border_width(&self) -> (Value, Value, Value, Value) {
        // border has initial value 0.
        let zero = Value::Length(0.0, Unit::Px);

        let mut border_top = self.value("border-top-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_bottom = self.value("border-bottom-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_left = self.value("border-left-width")
            .and_then(|x| Some(x[0].clone()));
        let mut border_right = self.value("border-right-width")
            .and_then(|x| Some(x[0].clone()));

        macro_rules! return_if_possible {
            () => {
                if border_top.is_some() && border_bottom.is_some()
                    && border_left.is_some() && border_right.is_some() {
                    return (
                        border_top.unwrap(), border_right.unwrap(),
                        border_bottom.unwrap(), border_left.unwrap(),
                    );
                }
            }
        }

        return_if_possible!();

        if let Some(border) = self.value("border-width") {
            match border.len() {
                1 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_bottom.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[0].clone());
                    border_right.get_or_insert_with(|| border[0].clone());
                }
                2 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_bottom.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[1].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                }
                3 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_left.get_or_insert_with(|| border[1].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                    border_bottom.get_or_insert_with(|| border[2].clone());
                }
                4 => {
                    border_top.get_or_insert_with(|| border[0].clone());
                    border_right.get_or_insert_with(|| border[1].clone());
                    border_bottom.get_or_insert_with(|| border[2].clone());
                    border_left.get_or_insert_with(|| border[3].clone());
                }
                0 | _ => unreachable!(),
            }
        }

        return_if_possible!();

        if let Some(border_info) = self.value("border-top") {
            let mut border_width = None;
            for border in border_info {
                if let &Value::Length(_, _) = &border {
                    border_width = Some(border);
                    break;
                }
            }
            if let Some(border_width) = border_width {
                border_top.get_or_insert_with(|| border_width.clone());
            }
        }

        if let Some(border_info) = self.value("border-bottom") {
            let mut border_width = None;
            for border in border_info {
                if let &Value::Length(_, _) = &border {
                    border_width = Some(border);
                    break;
                }
            }
            if let Some(border_width) = border_width {
                border_bottom.get_or_insert_with(|| border_width.clone());
            }
        }

        return_if_possible!();

        if let Some(border_info) = self.value("border") {
            let mut border_width = None;
            for border in border_info {
                if let &Value::Length(_, _) = &border {
                    border_width = Some(border);
                    break;
                }
            }
            if let Some(border_width) = border_width {
                border_top.get_or_insert_with(|| border_width.clone());
                border_right.get_or_insert_with(|| border_width.clone());
                border_bottom.get_or_insert_with(|| border_width.clone());
                border_left.get_or_insert_with(|| border_width.clone());
            }
        }

        border_top.get_or_insert_with(|| zero.clone());
        border_right.get_or_insert_with(|| zero.clone());
        border_bottom.get_or_insert_with(|| zero.clone());
        border_left.get_or_insert_with(|| zero.clone());

        (
            border_top.unwrap(),
            border_right.unwrap(),
            border_bottom.unwrap(),
            border_left.unwrap(),
        )
    }

    pub fn border_color(&self) -> (Option<Color>, Option<Color>, Option<Color>, Option<Color>) {
        let mut border_top = self.value("border-top-color").and_then(|x| x[0].to_color());
        let mut border_bottom = self.value("border-bottom-color")
            .and_then(|x| x[0].to_color());
        let mut border_left = self.value("border-left-color")
            .and_then(|x| x[0].to_color());
        let mut border_right = self.value("border-right-color")
            .and_then(|x| x[0].to_color());

        macro_rules! return_if_possible {
            () => {
                if border_top.is_some() && border_bottom.is_some()
                    && border_left.is_some() && border_right.is_some() {
                    return (
                        border_top, border_right,
                        border_bottom, border_left,
                    );
                }
            }
        }

        if let Some(border) = self.value("border-color") {
            match border.len() {
                1 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[0].to_color().unwrap());
                }
                2 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                }
                3 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[2].to_color().unwrap());
                }
                4 => {
                    border_top.get_or_insert_with(|| border[0].to_color().unwrap());
                    border_right.get_or_insert_with(|| border[1].to_color().unwrap());
                    border_bottom.get_or_insert_with(|| border[2].to_color().unwrap());
                    border_left.get_or_insert_with(|| border[3].to_color().unwrap());
                }
                0 | _ => unreachable!(),
            }
        }

        return_if_possible!();

        if let Some(border_info) = self.value("border-bottom") {
            if let Some(border_color) = (|| {
                for border in border_info {
                    let color = border.to_color();
                    if color.is_some() {
                        return color;
                    }
                }
                None
            })()
            {
                border_bottom.get_or_insert_with(|| border_color.clone());
            }
        }

        return_if_possible!();

        if let Some(border_info) = self.value("border") {
            if let Some(border_color) = (|| {
                for border in border_info {
                    let color = border.to_color();
                    if color.is_some() {
                        return color;
                    }
                }
                None
            })()
            {
                border_top.get_or_insert_with(|| border_color.clone());
                border_right.get_or_insert_with(|| border_color.clone());
                border_bottom.get_or_insert_with(|| border_color.clone());
                border_left.get_or_insert_with(|| border_color.clone());
            }
        }

        (border_top, border_right, border_bottom, border_left)
    }

    pub fn text_decoration(&self) -> Vec<TextDecoration> {
        if let Some(text_decorations) = self.value("text-decoration") {
            let mut decorations = vec![];
            for text_decoration in text_decorations {
                if let Some(decoration) = text_decoration.to_text_decoration() {
                    decorations.push(decoration);
                }
            }
            decorations
        } else {
            vec![]
        }
    }

    pub fn font_size(&self) -> Au {
        let default_font_size = Value::Length(DEFAULT_FONT_SIZE, Unit::Px);
        Au::from_f64_px(
            self.value_with_default("font-size", &vec![default_font_size])[0]
                .to_px()
                .unwrap(),
        )
    }

    pub fn font_weight(&self) -> FontWeight {
        let default_font_weight = Value::Keyword("normal".to_string());
        self.value_with_default("font-weight", &vec![default_font_weight])[0].to_font_weight()
    }

    pub fn font_style(&self) -> FontSlant {
        let default_font_slant = Value::Keyword("normal".to_string());
        self.lookup("font-style", "font-style", &vec![default_font_slant])[0].to_font_slant()
    }

    pub fn line_height(&self) -> Au {
        let font_size = self.font_size().to_f64_px();
        let default_line_height = Value::Length(font_size * DEFAULT_LINE_HEIGHT_SCALE, Unit::Px);
        let line_height = &self.value_with_default("line-height", &vec![default_line_height])[0];
        Au::from_f64_px(match line_height {
            &Value::Keyword(ref k) if k == "normal" => font_size * DEFAULT_LINE_HEIGHT_SCALE,
            &Value::Length(f, Unit::Px) => f,
            &Value::Length(f, Unit::Pt) => pt2px(f),
            &Value::Length(_, _) => unimplemented!(),
            &Value::Num(f) => font_size * f,
            _ => panic!(),
        })
    }

    pub fn text_align(&self) -> Value {
        self.value_with_default("text-align", &vec![Value::Keyword("left".to_string())])[0].clone()
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
    pub fn to_font_slant(&self) -> FontSlant {
        match self {
            &Value::Keyword(ref k) if k.as_str() == "normal" => FontSlant::Normal,
            &Value::Keyword(ref k) if k.as_str() == "italic" => FontSlant::Italic,
            _ => FontSlant::Normal,
        }
    }
}

#[test]
fn test1() {
    use html;
    use css;
    use std::path::Path;
    use default_style::*;

    let src = "<html>
                 <head>
                 </head>
                 <body style='font-size:10px;'>
                   <div id=\"x\">test</div>
                   <p>paragrapgh</p>
                   <span style='color:red;'>aa</span>
                   <a>link</a>
                   space
                 </body>
               </html>";
    let dom_node = html::parse(src.to_string(), Path::new("a.html").to_path_buf());

    let src = "* { display: block; }
               div, body > div, body span { width: 100px; height: 50px; color: #ffffff; background-color: #003300; } 
               a { display: inline; text-decoration: underline; }";
    css::parse(src.to_string());
}
