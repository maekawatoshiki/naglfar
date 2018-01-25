extern crate cairo;
extern crate gtk;

use gtk::traits::*;
use gtk::Inhibit;
// use gtk::{ContainerExt, WidgetExt, Window};
use cairo::Context;

use painter::{DisplayCommand, DisplayList};
use layout::Dimensions;

use layout::DEFAULT_FONT_SIZE;

struct RenderingWindow {
    window: gtk::Window,
    drawing_area: gtk::DrawingArea,
}

impl RenderingWindow {
    // TODO: Make this function receive a closure.
    // To use font-related info when laying out, layout.rs needs CairoContext.
    fn new<F: 'static>(width: i32, height: i32, f: F) -> RenderingWindow
    where
        F: Fn(&Context) -> DisplayList,
    {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Naglfar");
        window.set_default_size(width, height);
        let drawing_area = gtk::DrawingArea::new();
        let scrolled_window = gtk::ScrolledWindow::new(None, None);
        drawing_area.set_size_request(width, height);
        scrolled_window.add_with_viewport(&drawing_area);
        window.add(&scrolled_window);

        let instance = RenderingWindow {
            window: window,
            drawing_area: drawing_area,
        };

        instance
            .drawing_area
            .connect_draw(move |widget, cairo_context| {
                let items = f(cairo_context);
                if let DisplayCommand::SolidColor(_, rect) = items[0] {
                    widget.set_size_request(width, rect.height.ceil_to_px())
                }
                for item in &items {
                    render_item(cairo_context, item);
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

fn render_item(ctx: &Context, item: &DisplayCommand) {
    match item {
        &DisplayCommand::SolidColor(ref color, rect) => {
            ctx.rectangle(
                rect.x.ceil_to_px() as f64,
                rect.y.ceil_to_px() as f64,
                rect.width.ceil_to_px() as f64,
                rect.height.ceil_to_px() as f64,
            );
            ctx.set_source_rgb(
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
            );
            // ctx.stroke_preserve();
            ctx.fill();
        }
        &DisplayCommand::Text(ref text, rect) => {
            ctx.save();
            ctx.set_font_size(DEFAULT_FONT_SIZE);
            let font_ascent = ctx.get_scaled_font().extents().ascent;
            ctx.move_to(
                rect.x.ceil_to_px() as f64,
                font_ascent + rect.y.ceil_to_px() as f64,
            );
            ctx.set_source_rgb(0.0, 0.0, 0.0);
            ctx.show_text(text.as_str());
            ctx.restore();
        }
    }
}

pub fn render<F: 'static>(viewport: &Dimensions, f: F)
where
    F: Fn(&Context) -> DisplayList,
{
    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let window = RenderingWindow::new(
        viewport.content.width.to_f64_px() as i32,
        viewport.content.height.to_f64_px() as i32,
        f,
    );
    window.exit_on_close();
    gtk::main();
}
