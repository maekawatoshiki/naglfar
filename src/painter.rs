use layout::{BoxType, LayoutBox, LayoutInfo, Rect};
use font::Font;
use dom::{ElementData, LayoutType, NodeType};
use css::{Color, BLACK};
use app_units::Au;

use gdk_pixbuf;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Image(gdk_pixbuf::Pixbuf, Rect),
    Text(String, Rect, Color, Font),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ContentType {
    Float,
    None,
}

#[derive(Debug, Clone)]
pub struct DisplayCommandInfo {
    pub command: DisplayCommand,
    pub content_type: ContentType,
    pub z_index: i32,
}

impl DisplayCommandInfo {
    pub fn new(
        command: DisplayCommand,
        z_index: i32,
        content_type: ContentType,
    ) -> DisplayCommandInfo {
        DisplayCommandInfo {
            command: command,
            z_index: z_index,
            content_type: content_type,
        }
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
        ContentType::None,
    );

    list.sort_by(
        |&DisplayCommandInfo { z_index: a, .. }, &DisplayCommandInfo { z_index: b, .. }| a.cmp(&b),
    );
    let mut ordered_list = Vec::new();
    for item in list.iter()
        .filter(|item| item.content_type == ContentType::None)
    {
        ordered_list.push(item.clone());
    }
    for item in list.iter()
        .filter(|item| item.content_type == ContentType::Float)
    {
        ordered_list.push(item.clone());
    }
    ordered_list
}

fn render_layout_box(
    list: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &LayoutBox,
    content_type: ContentType,
) {
    let content_type = if content_type == ContentType::None && layout_box.box_type == BoxType::Float
    {
        ContentType::Float
    } else {
        content_type
    };

    render_background(list, x, y, layout_box, content_type);
    render_borders(list, x, y, layout_box, content_type);

    let children = layout_box.children.clone();
    for child in children {
        render_layout_box(
            list,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &child,
            content_type,
        );
    }

    render_text(list, x, y, layout_box, content_type);
    render_image(list, x, y, layout_box, content_type);
}

fn render_text(
    list: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &LayoutBox,
    content_type: ContentType,
) {
    if let &BoxType::TextNode(ref text_info) = &layout_box.box_type {
        let text = if let NodeType::Text(ref text) = layout_box.style.unwrap().node.data {
            &text.as_str()[text_info.range.clone()]
        } else {
            unreachable!()
        };
        list.push(DisplayCommandInfo::new(
            DisplayCommand::Text(
                text.to_string(),
                layout_box.dimensions.content.add_parent_coordinate(x, y),
                get_color(layout_box, "color").unwrap_or(BLACK),
                text_info.font,
            ),
            layout_box.z_index,
            content_type,
        ));
    }
}

fn render_image(
    list: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &LayoutBox,
    content_type: ContentType,
) {
    match layout_box.box_type {
        BoxType::InlineNode | BoxType::Float => {
            if let NodeType::Element(ElementData {
                ref layout_type, ..
            }) = layout_box.style.unwrap().node.data
            {
                if layout_type == &LayoutType::Image {
                    list.push(DisplayCommandInfo::new(
                        DisplayCommand::Image(
                            if let &LayoutInfo::Image(ref pixbuf) = &layout_box.info {
                                pixbuf.clone()
                            } else {
                                panic!()
                            },
                            layout_box.dimensions.content.add_parent_coordinate(x, y),
                        ),
                        layout_box.z_index,
                        content_type,
                    ))
                }
            }
        }
        _ => {}
    }
}

fn render_background(
    list: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &LayoutBox,
    content_type: ContentType,
) {
    lookup_color(layout_box, "background-color", "background").map(|color| {
        list.push(DisplayCommandInfo::new(
            DisplayCommand::SolidColor(
                color,
                layout_box
                    .dimensions
                    .border_box()
                    .add_parent_coordinate(x, y),
            ),
            layout_box.z_index,
            content_type,
        ))
    });
}

fn render_borders(
    list: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &LayoutBox,
    content_type: ContentType,
) {
    let d = &layout_box.dimensions;
    let border_box = d.border_box().add_parent_coordinate(x, y);

    // Left border
    if let Some(left_color) = lookup_color(layout_box, "border-left-color", "border-color") {
        list.push(DisplayCommandInfo::new(
            DisplayCommand::SolidColor(
                left_color,
                Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: d.border.left,
                    height: border_box.height,
                },
            ),
            layout_box.z_index,
            content_type,
        ));
    }

    // Right border
    if let Some(right_color) = lookup_color(layout_box, "border-right-color", "border-color") {
        list.push(DisplayCommandInfo::new(
            DisplayCommand::SolidColor(
                right_color,
                Rect {
                    x: border_box.x + border_box.width - d.border.right,
                    y: border_box.y,
                    width: d.border.right,
                    height: border_box.height,
                },
            ),
            layout_box.z_index,
            content_type,
        ));
    }

    // Top border
    if let Some(top_color) = lookup_color(layout_box, "border-top-color", "border-color") {
        list.push(DisplayCommandInfo::new(
            DisplayCommand::SolidColor(
                top_color,
                Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: border_box.width,
                    height: d.border.top,
                },
            ),
            layout_box.z_index,
            content_type,
        ));
    }

    // Bottom border
    if let Some(bottom_color) = lookup_color(layout_box, "border-bottom-color", "border-color") {
        list.push(DisplayCommandInfo::new(
            DisplayCommand::SolidColor(
                bottom_color,
                Rect {
                    x: border_box.x,
                    y: border_box.y + border_box.height - d.border.bottom,
                    width: border_box.width,
                    height: d.border.bottom,
                },
            ),
            layout_box.z_index,
            content_type,
        ));
    }
}

/// Return the specified color for CSS property `name`, or None if no color was specified.
fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match layout_box.style {
        Some(style) => match style.value(name) {
            Some(maybe_color) => maybe_color.to_color(),
            _ => None,
        },
        None => None,
    }
}

/// Return the specified color for CSS property `name` or `fallback_name`, or None if no color was specified.
fn lookup_color(layout_box: &LayoutBox, name: &str, fallback_name: &str) -> Option<Color> {
    match layout_box.style {
        Some(style) => match style.lookup_without_default(name, fallback_name) {
            Some(maybe_color) => maybe_color.to_color(),
            _ => None,
        },
        None => None,
    }
}
