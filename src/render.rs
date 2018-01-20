use painter::{DisplayCommand, DisplayList};
use layout::{Dimensions, Rect};
use std::io::Result;

use printpdf::*;

use std::fs::File;
use std::io::BufWriter;

pub fn render(items: DisplayList, viewport: &Dimensions) {
    let (doc, page1, layer1) = PdfDocument::new(
        "printpdf graphics test",
        viewport.content.width,
        viewport.content.height,
        "Layer",
    );
    let current_layer = doc.get_page(page1).get_layer(layer1);

    for item in items {
        render_item(&doc, &current_layer, &item, viewport);
    }

    // If this is successful, you should see a PDF two shapes, one rectangle
    // and a dotted line
    doc.save(&mut BufWriter::new(File::create("test.pdf").unwrap()))
        .unwrap();
}

fn render_item(
    doc: &types::PdfDocumentReference,
    layer: &types::pdf_layer::PdfLayerReference,
    item: &DisplayCommand,
    viewport: &Dimensions,
) {
    match item {
        &DisplayCommand::SolidColor(ref color, rect) => {
            let points1 = vec![
                (Point::new(rect.x, 360.0 - rect.y), false),
                (Point::new(rect.x, 360.0 - (rect.y + rect.height)), false),
                (
                    Point::new(rect.x + rect.width, 360.0 - (rect.y + rect.height)),
                    false,
                ),
                (Point::new(rect.x + rect.width, 360.0 - rect.y), false),
            ];
            let line1 = Line::new(points1, true, true, true);
            let fill_color = Color::Rgb(Rgb::new(
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
                None,
            ));
            layer.set_fill_color(fill_color);
            layer.add_shape(line1);
        }
        &DisplayCommand::Text(ref text, rect) => {
            let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();

            layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            // text, font size, x from left edge, y from top edge, font
            layer.use_text(
                text.as_str(),
                16 * 3,
                rect.x,
                360.0 - rect.y - rect.height,
                &font,
            );

            // For more complex layout of text, you can use functions
            // defined on the PdfLayerReference
            // Make sure to wrap your commands
            // in a `begin_text_section()` and `end_text_section()` wrapper
            // layer.begin_text_section();

            // setup the general fonts.
            // see the docs for these functions for details
            // layer.set_font(&font, 16);
            // current_layer.set_text_cursor(10.0, 10.0);
            // layer.set_line_height(16);
            // current_layer.set_word_spacing(3000);
            // current_layer.set_character_spacing(10);
            // layer.set_text_rendering_mode(TextRenderingMode::Stroke);
            //
            // // write two lines (one line break)
            // layer.write_text(text.clone(), &font);
            // current_layer.add_line_break();
            // current_layer.write_text(text2.clone(), &font2);
            // current_layer.add_line_break();
            //
            // // write one line, but write text2 in superscript
            // current_layer.write_text(text.clone(), &font2);
            // current_layer.set_line_offset(10);
            // current_layer.write_text(text2.clone(), &font2);
        }
    }
}
//
// pub fn get_str_width(s: &str) -> f32 {
//     BuiltinFont::Helvetica.get_width(16.0, s)
// }
//
// impl Dimensions {
//     fn y(&self, rect: Rect) -> f32 {
//         self.content.height as f32 - rect.y as f32 - rect.height as f32
//     }
// }
