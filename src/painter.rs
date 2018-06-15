use layout::{BoxType, ImageMetaData, LayoutBox, LayoutInfo, Rect};
use font::Font;
use dom::{ElementData, LayoutType, NodeType};
use css::{Color, TextDecoration, BLACK};
use app_units::Au;

use gdk_pixbuf;
use gtk;

use window::{AnkerKind, ANKERS, URL_FRAGMENTS};

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Image(gdk_pixbuf::Pixbuf, ImageMetaData, Rect),
    Text(String, Rect, Color, Vec<TextDecoration>, Font),
    Button(gtk::Button, Rect),
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

pub fn build_display_list(layout_root: &mut LayoutBox) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(
        &mut list,
        Au::from_f64_px(0.0),
        Au::from_f64_px(0.0),
        layout_root,
    );
    list
}

fn render_layout_box(list: &mut DisplayList, x: Au, y: Au, layout_box: &mut LayoutBox) {
    let is_input_elem = match layout_box.info {
        LayoutInfo::Button(_, _) => true,
        _ => false,
    };

    let mut buf = DisplayList::new();

    render_background(&mut buf, x, y, layout_box);
    render_borders(&mut buf, x, y, layout_box);

    let mut children = layout_box.children.clone();
    children.sort_by(|&LayoutBox { z_index: a, .. }, &LayoutBox { z_index: b, .. }| a.cmp(&b));

    for mut child in children
        .iter_mut()
        .filter(|child| child.box_type != BoxType::Float)
    {
        render_layout_box(
            &mut buf,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &mut child,
        );
    }
    for mut child in children
        .iter_mut()
        .filter(|child| child.box_type == BoxType::Float)
    {
        render_layout_box(
            &mut buf,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &mut child,
        );
    }

    render_text(&mut buf, x, y, layout_box);
    render_image(&mut buf, x, y, layout_box);

    register_anker(x, y, layout_box);
    register_url_fragment(x, y, layout_box);

    if is_input_elem {
        render_button(list, &mut buf, x, y, layout_box);
    } else {
        list.append(&mut buf);
    }
}

fn render_button(
    list: &mut DisplayList,
    _children: &mut DisplayList,
    x: Au,
    y: Au,
    layout_box: &mut LayoutBox,
) {
    if let &LayoutInfo::Button(ref btn, _) = &layout_box.info {
        list.push(DisplayCommandInfo::new(DisplayCommand::Button(
            btn.clone().unwrap(),
            layout_box.dimensions.content.add_parent_coordinate(x, y),
        )));
    }
}

fn render_text(list: &mut DisplayList, x: Au, y: Au, layout_box: &mut LayoutBox) {
    if let &mut BoxType::TextNode(ref text_info) = &mut layout_box.box_type {
        let text = if let NodeType::Text(ref text) = layout_box.node.data {
            &text.as_str()[text_info.range.clone()]
        } else {
            unreachable!()
        };
        list.push(DisplayCommandInfo::new(DisplayCommand::Text(
            text.to_string(),
            layout_box.dimensions.content.add_parent_coordinate(x, y),
            match layout_box.property.value("color") {
                Some(maybe_color) => maybe_color[0].to_color(),
                _ => None,
            }.unwrap_or(BLACK),
            layout_box.property.text_decoration(),
            text_info.font,
        )));
    }
}

fn render_image(list: &mut DisplayList, x: Au, y: Au, layout_box: &mut LayoutBox) {
    if let NodeType::Element(ElementData {
        ref layout_type, ..
    }) = layout_box.node.data
    {
        if layout_type == &LayoutType::Image {
            if let &LayoutInfo::Image(ref imgdata) = &layout_box.info {
                list.push(DisplayCommandInfo::new(DisplayCommand::Image(
                    imgdata.pixbuf.clone().unwrap(),
                    imgdata.metadata.clone(),
                    layout_box.dimensions.content.add_parent_coordinate(x, y),
                )))
            }
        }
    }
}

fn register_anker(x: Au, y: Au, layout_box: &mut LayoutBox) {
    match layout_box.info {
        LayoutInfo::Anker => {
            if let Some(url) = layout_box.node.anker_url() {
                let rect = layout_box.dimensions.content.add_parent_coordinate(x, y);
                ANKERS.with(|ankers| {
                    ankers.borrow_mut().entry(rect).or_insert_with(|| {
                        if url.chars().next().unwrap() == '#' {
                            AnkerKind::URLFragment(url[1..].to_string())
                        } else {
                            AnkerKind::URL(url.to_string())
                        }
                    });
                });
            }
        }
        _ => {}
    }
}

fn register_url_fragment(x: Au, y: Au, layout_box: &mut LayoutBox) {
    if let NodeType::Element(ref e) = layout_box.node.data {
        if let Some(id) = e.id() {
            URL_FRAGMENTS.with(|url_fragments| {
                url_fragments.borrow_mut().insert(
                    id.to_string(),
                    layout_box
                        .dimensions
                        .content
                        .add_parent_coordinate(x, y)
                        .y
                        .to_f64_px(),
                )
            });
        }
    }
}

fn render_background(list: &mut DisplayList, x: Au, y: Au, layout_box: &mut LayoutBox) {
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

fn render_borders(list: &mut DisplayList, x: Au, y: Au, layout_box: &mut LayoutBox) {
    let d = &layout_box.dimensions;
    let border_box = d.border_box().add_parent_coordinate(x, y);

    let (top_color, right_color, bottom_color, left_color) = layout_box.property.border_color();

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

/// Return the specified color for CSS property `name` or `fallback_name`, or None if no color was specified.
fn lookup_color(layout_box: &mut LayoutBox, name: &str, fallback_name: &str) -> Option<Color> {
    match layout_box
        .property
        .lookup_without_default(name, fallback_name)
    {
        Some(maybe_color) => maybe_color[0].to_color(),
        _ => None,
    }
}
