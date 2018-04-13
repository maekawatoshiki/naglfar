use layout::{BoxType, LayoutBox, LayoutInfo, Rect};
use font::Font;
use dom::{ElementData, LayoutType, NodeType};
use css::{Color, TextDecoration, BLACK};
use app_units::Au;

use gdk_pixbuf;

use window::{AnkerKind, ANKERS, URL_FRAGMENTS};

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Image(gdk_pixbuf::Pixbuf, Rect),
    Text(String, Rect, Color, Vec<TextDecoration>, Font),
}

#[derive(Debug, Clone)]
pub struct DisplayCommandInfo {
    pub command: DisplayCommand,
}

impl DisplayCommandInfo {
    pub fn new(command: DisplayCommand) -> DisplayCommandInfo {
        DisplayCommandInfo { command: command }
    }
}

pub type DisplayList = Vec<DisplayCommandInfo>;

pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(
        &mut list,
        Au::from_f64_px(0.0),
        Au::from_f64_px(0.0),
        layout_root,
    );
    list
}

fn render_layout_box(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    render_background(list, x, y, layout_box);
    render_borders(list, x, y, layout_box);

    let mut children = layout_box.children.clone();
    children.sort_by(|&LayoutBox { z_index: a, .. }, &LayoutBox { z_index: b, .. }| a.cmp(&b));

    for child in children
        .iter()
        .filter(|child| child.box_type != BoxType::Float)
    {
        render_layout_box(
            list,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &child,
        );
    }
    for child in children
        .iter()
        .filter(|child| child.box_type == BoxType::Float)
    {
        render_layout_box(
            list,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &child,
        );
    }

    render_text(list, x, y, layout_box);
    render_image(list, x, y, layout_box);

    register_anker(x, y, layout_box);
    register_url_fragment(x, y, layout_box);
}

fn render_text(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    if let &BoxType::TextNode(ref text_info) = &layout_box.box_type {
        let text = if let NodeType::Text(ref text) = layout_box.style.unwrap().node.data {
            &text.as_str()[text_info.range.clone()]
        } else {
            unreachable!()
        };
        list.push(DisplayCommandInfo::new(DisplayCommand::Text(
            text.to_string(),
            layout_box.dimensions.content.add_parent_coordinate(x, y),
            get_color(layout_box, "color").unwrap_or(BLACK),
            match layout_box.style {
                Some(style) => style.text_decoration(),
                None => vec![],
            },
            text_info.font,
        )));
    }
}

fn render_image(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    match layout_box.box_type {
        BoxType::InlineNode | BoxType::Float => {
            if let NodeType::Element(ElementData {
                ref layout_type, ..
            }) = layout_box.style.unwrap().node.data
            {
                if layout_type == &LayoutType::Image {
                    list.push(DisplayCommandInfo::new(DisplayCommand::Image(
                        if let &LayoutInfo::Image(ref pixbuf) = &layout_box.info {
                            pixbuf.clone().unwrap()
                        } else {
                            panic!()
                        },
                        layout_box.dimensions.content.add_parent_coordinate(x, y),
                    )))
                }
            }
        }
        _ => {}
    }
}

fn register_anker(x: Au, y: Au, layout_box: &LayoutBox) {
    match layout_box.info {
        LayoutInfo::Anker => {
            if let Some(url) = layout_box.style.unwrap().node.anker_url() {
                let rect = layout_box.dimensions.content.add_parent_coordinate(x, y);
                ANKERS.with(|ankers| {
                    ankers
                        .borrow_mut()
                        .entry(rect)
                        .or_insert_with(|| {
                            if url.chars().next().unwrap() == '#' {
                                AnkerKind::URLFragment(url[1..].to_string())
                            } else {
                                AnkerKind::URL(url.to_string())
                            }
                        })
                        .clone()
                });
            }
        }
        _ => {}
    }
}

fn register_url_fragment(x: Au, y: Au, layout_box: &LayoutBox) {
    if let Some(style) = layout_box.style {
        if let NodeType::Element(ref e) = style.node.data {
            if let Some(id) = e.id() {
                URL_FRAGMENTS.with(|url_fragments| {
                    url_fragments
                        .borrow_mut()
                        .entry(id.to_string())
                        .or_insert_with(|| {
                            layout_box
                                .dimensions
                                .content
                                .add_parent_coordinate(x, y)
                                .y
                                .to_f64_px()
                        })
                        .clone()
                });
            }
        }
    }
}

fn render_background(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    lookup_color(layout_box, "background-color", "background").map(|color| {
        list.push(DisplayCommandInfo::new(DisplayCommand::SolidColor(
            color,
            layout_box
                .dimensions
                .border_box()
                .add_parent_coordinate(x, y),
        )))
    });
}

fn render_borders(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    let d = &layout_box.dimensions;
    let border_box = d.border_box().add_parent_coordinate(x, y);

    let (top_color, right_color, bottom_color, left_color) = match layout_box.style {
        Some(style) => style.border_color(),
        None => return,
    };

    // Left border
    if let Some(left_color) = left_color {
        list.push(DisplayCommandInfo::new(DisplayCommand::SolidColor(
            left_color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: d.border.left,
                height: border_box.height,
            },
        )));
    }

    // Right border
    if let Some(right_color) = right_color {
        list.push(DisplayCommandInfo::new(DisplayCommand::SolidColor(
            right_color,
            Rect {
                x: border_box.x + border_box.width - d.border.right,
                y: border_box.y,
                width: d.border.right,
                height: border_box.height,
            },
        )));
    }

    // Top border
    if let Some(top_color) = top_color {
        list.push(DisplayCommandInfo::new(DisplayCommand::SolidColor(
            top_color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: d.border.top,
            },
        )));
    }

    // Bottom border
    if let Some(bottom_color) = bottom_color {
        list.push(DisplayCommandInfo::new(DisplayCommand::SolidColor(
            bottom_color,
            Rect {
                x: border_box.x,
                y: border_box.y + border_box.height - d.border.bottom,
                width: border_box.width,
                height: d.border.bottom,
            },
        )));
    }
}

/// Return the specified color for CSS property `name`, or None if no color was specified.
fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match layout_box.style {
        Some(style) => match style.value(name) {
            Some(maybe_color) => maybe_color[0].to_color(),
            _ => None,
        },
        None => None,
    }
}

/// Return the specified color for CSS property `name` or `fallback_name`, or None if no color was specified.
fn lookup_color(layout_box: &LayoutBox, name: &str, fallback_name: &str) -> Option<Color> {
    match layout_box.style {
        Some(style) => match style.lookup_without_default(name, fallback_name) {
            Some(maybe_color) => maybe_color[0].to_color(),
            _ => None,
        },
        None => None,
    }
}
