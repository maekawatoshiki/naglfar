use layout::{BoxType, Dimensions, LayoutBox, Rect};
use css::{Color, Value};
use dom::NodeType;
use cairo::Context;
use app_units::Au;

use layout::DEFAULT_FONT_SIZE;
use layout::DEFAULT_LINE_HEIGHT;

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Text(String, Rect),
}

pub type DisplayList = Vec<DisplayCommand>;

pub fn build_display_list(
    ctx: &Context,
    containing_block: Dimensions,
    layout_root: &LayoutBox,
) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(&mut list, ctx, containing_block, layout_root);
    list
}

fn render_layout_box(
    list: &mut DisplayList,
    ctx: &Context,
    containing_block: Dimensions,
    layout_box: &LayoutBox,
) {
    render_text(list, ctx, containing_block, layout_box);
    render_background(list, layout_box);
    render_borders(list, layout_box);
    for child in &layout_box.children {
        render_layout_box(list, ctx, layout_box.dimensions, child);
    }
}

fn render_text(
    list: &mut DisplayList,
    ctx: &Context,
    containing_block: Dimensions,
    layout_box: &LayoutBox,
) {
    match layout_box.box_type {
        BoxType::BlockNode(node) | BoxType::InlineNode(node) => match node.node.data {
            NodeType::Element(_) => (),
            NodeType::Text(ref text) => {
                ctx.set_font_size(DEFAULT_FONT_SIZE);
                let font_info = ctx.get_scaled_font();
                let text_width = font_info.text_extents(text.as_str()).x_advance;
                let font_width = font_info.extents().max_x_advance;
                let max_width = containing_block.content.width;
                if max_width.to_f64_px() > 0.0 && max_width.to_f64_px() < text_width {
                    let mut line = "".to_string();
                    let mut d = layout_box.dimensions;
                    for c in text.chars() {
                        line.push(c);
                        if max_width.to_f64_px()
                            - (d.content.x.to_f64_px() - containing_block.content.x.to_f64_px())
                            - font_width
                            < font_info.text_extents(line.as_str()).x_advance
                        {
                            list.push(DisplayCommand::Text(line.clone(), d.border_box()));
                            d.content.x = containing_block.content.x;
                            d.content.y += Au::from_f64_px(DEFAULT_LINE_HEIGHT);
                            line.clear();
                        }
                    }
                    list.push(DisplayCommand::Text(line, d.border_box()))
                } else {
                    list.push(DisplayCommand::Text(
                        text.clone(),
                        layout_box.dimensions.border_box(),
                    ))
                };
            }
        },
        _ => (),
    }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox) {
    get_color(layout_box, "background").map(|color| {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.border_box(),
        ))
    });
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        _ => return,
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

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
        BoxType::BlockNode(style) | BoxType::InlineNode(style) => match style.value(name) {
            Some(Value::Color(color)) => Some(color),
            _ => None,
        },
        BoxType::AnonymousBlock => None,
    }
}
