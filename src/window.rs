extern crate cairo;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate pango;
extern crate pangocairo;

use gtk::traits::*;
use gtk::Inhibit;

use gdk::ContextExt;

use cairo::Context;
use pango::LayoutExt;

use painter::{DisplayCommand, DisplayCommandInfo, DisplayList};
use font::FONT_DESC;

struct RenderingWindow {
    window: gtk::Window,
    drawing_area: gtk::DrawingArea,
}

impl RenderingWindow {
    fn new<F: 'static>(width: i32, height: i32, f: F) -> RenderingWindow
    where
        F: Fn(&gtk::DrawingArea) -> DisplayList,
    {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Naglfar");
        window.set_default_size(width, height);

        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(width, height);

        let scrolled_window = gtk::ScrolledWindow::new(None, None);
        scrolled_window.add_with_viewport(&drawing_area);

        window.add(&scrolled_window);

        let instance = RenderingWindow {
            window: window,
            drawing_area: drawing_area,
        };

        instance
            .drawing_area
            .connect_draw(move |widget, cairo_context| {
                let pango_ctx = widget.create_pango_context().unwrap();
                let mut pango_layout = pango::Layout::new(&pango_ctx);

                let mut items = f(widget);
                if let DisplayCommand::SolidColor(_, rect) = items[0].command {
                    widget.set_size_request(width, rect.height.ceil_to_px())
                }

                items.sort_by(
                    |&DisplayCommandInfo { z_index: a, .. },
                     &DisplayCommandInfo { z_index: b, .. }| { a.cmp(&b) },
                );
                for item in &items {
                    render_item(cairo_context, &mut pango_layout, &item.command);
                }
                Inhibit(true)
            });
        instance.window.show_all();
        instance
    }

    fn exit_on_close(&self) {
        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(true)
        });
    }
}

fn render_item(ctx: &Context, pango_layout: &mut pango::Layout, item: &DisplayCommand) {
    match item {
        &DisplayCommand::SolidColor(ref color, rect) => {
            ctx.rectangle(
                rect.x.to_px() as f64,
                rect.y.to_px() as f64,
                rect.width.to_px() as f64,
                rect.height.to_px() as f64,
            );
            ctx.set_source_rgba(
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
                color.a as f64 / 255.0,
            );
            ctx.fill();
        }
        &DisplayCommand::Image(ref pixbuf, rect) => {
            ctx.set_source_pixbuf(&pixbuf, rect.x.to_f64_px(), rect.y.to_f64_px());
            ctx.paint();
        }
        &DisplayCommand::Text(ref text, rect, ref color, ref font) => {
            FONT_DESC.with(|font_desc| {
                font_desc
                    .borrow_mut()
                    .set_size(pango::units_from_double(font.size.to_f64_px() * 0.752)); // px to pt. TODO: Fix this!
                font_desc
                    .borrow_mut()
                    .set_style(font.slant.to_pango_font_slant());
                font_desc
                    .borrow_mut()
                    .set_weight(font.weight.to_pango_font_weight());
                pango_layout.set_text(text.as_str());
                pango_layout.set_font_description(Some(&*font_desc.borrow()));
            });

            ctx.set_source_rgba(
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
                color.a as f64 / 255.0,
            );
            ctx.move_to(rect.x.to_px() as f64, rect.y.to_px() as f64);

            pango_layout.context_changed();
            pangocairo::functions::show_layout(ctx, &pango_layout);
        }
    }
}

pub fn render<F: 'static>(f: F)
where
    F: Fn(&gtk::DrawingArea) -> DisplayList,
{
    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let window = RenderingWindow::new(640, 480, f);
    window.exit_on_close();

    gtk::main();
}
