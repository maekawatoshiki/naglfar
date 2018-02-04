use layout::{BoxType, Font, LayoutBox, Rect, Text};
use css::{Color, Value};
use app_units::Au;

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Text(String, Rect, Font),
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
    render_text(
        list,
        x + layout_box.dimensions.content.x,
        y + layout_box.dimensions.content.y,
        layout_box,
    );
}

fn render_text(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    if let &BoxType::AnonymousBlock(ref texts) = &layout_box.box_type {
        for &Text {
            ref rect,
            ref text,
            ref font,
            ref line_height,
        } in texts
        {
            list.push(DisplayCommand::Text(
                text.clone(),
                rect.add_xy(x, y),
                font.clone(),
            ));
        }
    }
}

fn render_background(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    get_color(layout_box, "background").map(|color| {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.border_box().add_xy(x, y),
        ))
    });
}

fn render_borders(list: &mut DisplayList, x: Au, y: Au, layout_box: &LayoutBox) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        _ => return,
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box().add_xy(x, y);

    // Left border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
    ));

    // Right border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
    ));

    // Top border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
    ));

    // Bottom border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    ));
}

/// Return the specified color for CSS property `name`, or None if no color was specified.
fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match layout_box.box_type {
        BoxType::BlockNode(ref style)
        | BoxType::InlineNode(ref style)
        | BoxType::TextNode(ref style) => match style.value(name) {
            Some(Value::Color(color)) => Some(color),
            _ => None,
        },
        BoxType::AnonymousBlock(_) => None,
    }
}
