use layout::{BoxType, LayoutBox, Rect};
use font::Font;
use dom::NodeType;
use css::{Color, BLACK};
use app_units::Au;

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Text(String, Rect, Color, Font),
}

pub type DisplayList = Vec<DisplayCommand>;

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
    for child in children {
        render_layout_box(
            list,
            x + layout_box.dimensions.content.x,
            y + layout_box.dimensions.content.y,
            &child,
        );
    }
    render_text(list, x, y, layout_box);
}

fn render_text(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    if let &BoxType::TextNode(ref text_info) = &layout_box.box_type {
        let text = if let NodeType::Text(ref text) = layout_box.style.unwrap().node.data {
            &text.as_str()[text_info.range.clone()]
        } else {
            unreachable!()
        };
        list.push(DisplayCommand::Text(
            text.to_string(),
            layout_box.dimensions.content.add_parent_coordinate(x, y),
            get_color(layout_box, "color").unwrap_or(BLACK),
            text_info.font,
        ));
    }
}

fn render_background(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    lookup_color(layout_box, "background-color", "background").map(|color| {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box
                .dimensions
                .border_box()
                .add_parent_coordinate(x, y),
        ))
    });
}

fn render_borders(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    let d = &layout_box.dimensions;
    let border_box = d.border_box().add_parent_coordinate(x, y);

    // Left border
    if let Some(left_color) = lookup_color(layout_box, "border-left-color", "border-color") {
        list.push(DisplayCommand::SolidColor(
            left_color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: d.border.left,
                height: border_box.height,
            },
        ));
    }

    // Right border
    if let Some(right_color) = lookup_color(layout_box, "border-right-color", "border-color") {
        list.push(DisplayCommand::SolidColor(
            right_color,
            Rect {
                x: border_box.x + border_box.width - d.border.right,
                y: border_box.y,
                width: d.border.right,
                height: border_box.height,
            },
        ));
    }

    // Top border
    if let Some(top_color) = lookup_color(layout_box, "border-top-color", "border-color") {
        list.push(DisplayCommand::SolidColor(
            top_color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: d.border.top,
            },
        ));
    }

    // Bottom border
    if let Some(bottom_color) = lookup_color(layout_box, "border-bottom-color", "border-color") {
        list.push(DisplayCommand::SolidColor(
            bottom_color,
            Rect {
                x: border_box.x,
                y: border_box.y + border_box.height - d.border.bottom,
                width: border_box.width,
                height: d.border.bottom,
            },
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
