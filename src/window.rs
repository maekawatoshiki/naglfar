extern crate cairo;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate pango;
extern crate pangocairo;

use gtk::{Inhibit, ObjectExt, WidgetExt, traits::*};

use glib::prelude::*; // or `use gtk::prelude::*;`

use gdk::{ContextExt, Cursor, CursorType, Event, EventButton, EventMask, EventMotion, WindowExt};
use gdk_pixbuf::{InterpType, PixbufExt};

use cairo::Context;
use pango::LayoutExt;

use std::{cell::RefCell, cmp::{max, min}, collections::HashMap};

use layout::Rect;
use painter::{DisplayCommand, DisplayList};
use font::FONT_DESC;
use css::{TextDecoration, px2pt};
use interface::update_html_tree_and_stylesheet;

#[derive(Clone, Debug)]
pub enum AnkerKind {
    URL(String),
    URLFragment(String),
}

thread_local!(
    pub static ANKERS: RefCell<HashMap<Rect, AnkerKind>> = { RefCell::new(HashMap::with_capacity(8)) };
    // HashMap<URL Fragment(id), y coordinate of the content
    pub static URL_FRAGMENTS: RefCell<HashMap<String, f64>> = { RefCell::new(HashMap::with_capacity(8)) }
);

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

        drawing_area.add_events(
            EventMask::POINTER_MOTION_MASK.bits() as i32
                | EventMask::BUTTON_PRESS_MASK.bits() as i32,
        );
        drawing_area
            .connect("motion-notify-event", false, |args| {
                let drawing_area = args[0]
                    .clone()
                    .downcast::<gtk::DrawingArea>()
                    .unwrap()
                    .get()
                    .unwrap();
                let (x, y) = args[1]
                    .clone()
                    .downcast::<Event>()
                    .unwrap()
                    .get()
                    .unwrap()
                    .downcast::<EventMotion>()
                    .unwrap()
                    .get_position();

                ANKERS.with(|ankers| {
                    let window = drawing_area.get_window().unwrap();
                    if (&*ankers.borrow()).iter().any(|(rect, _)| {
                        rect.x.to_f64_px() <= x && x <= rect.x.to_f64_px() + rect.width.to_f64_px()
                            && rect.y.to_f64_px() <= y
                            && y <= rect.y.to_f64_px() + rect.height.to_f64_px()
                    }) {
                        window.set_cursor(Some(&Cursor::new(CursorType::Hand1)));
                    } else {
                        // TODO: This is executed many times. It's inefficient.
                        window.set_cursor(Some(&Cursor::new(CursorType::LeftPtr)));
                    }
                });
                Some(true.to_value())
            })
            .unwrap();

        drawing_area
            .connect("button-press-event", false, |args| {
                let drawing_area = args[0]
                    .clone()
                    .downcast::<gtk::DrawingArea>()
                    .unwrap()
                    .get()
                    .unwrap();
                let (clicked_x, clicked_y) = args[1]
                    .clone()
                    .downcast::<Event>()
                    .unwrap()
                    .get()
                    .unwrap()
                    .downcast::<EventButton>()
                    .unwrap()
                    .get_position();

                ANKERS.with(|ankers| {
                    // TODO: Makes no sense.
                    let mut ankers = ankers.borrow_mut();
                    let mut anker_clicked = false;
                    if let Some((_, ankerkind)) = ankers.iter().find(|&(rect, _)| {
                        rect.x.to_f64_px() <= clicked_x
                            && clicked_x <= rect.x.to_f64_px() + rect.width.to_f64_px()
                            && rect.y.to_f64_px() <= clicked_y
                            && clicked_y <= rect.y.to_f64_px() + rect.height.to_f64_px()
                    }) {
                        match ankerkind {
                            &AnkerKind::URL(ref url) => {
                                anker_clicked = true;
                                update_html_tree_and_stylesheet(url.to_string());
                                args[0]
                                    .clone()
                                    .downcast::<gtk::DrawingArea>()
                                    .unwrap()
                                    .get()
                                    .unwrap()
                                    .queue_draw();
                            }
                            &AnkerKind::URLFragment(ref id) => {
                                URL_FRAGMENTS.with(|ufs| {
                                    if let Some(content_y) = ufs.borrow().get(id) {
                                        let mut adjustment = drawing_area
                                            .get_parent()
                                            .unwrap()
                                            .get_parent()
                                            .unwrap()
                                            .downcast::<gtk::ScrolledWindow>()
                                            .unwrap()
                                            .get_vadjustment()
                                            .unwrap();
                                        adjustment.set_value(*content_y);
                                    }
                                });
                            }
                        };
                    }
                    if anker_clicked {
                        ankers.clear()
                    }
                });
                Some(true.to_value())
            })
            .unwrap();

        let instance = RenderingWindow {
            window: window,
            drawing_area: drawing_area,
        };

        instance
            .drawing_area
            .connect_draw(move |widget, cairo_context| {
                let (_, redraw_start_y, _, redraw_end_y) = cairo_context.clip_extents();
                let pango_ctx = widget.create_pango_context().unwrap();
                let mut pango_layout = pango::Layout::new(&pango_ctx);

                let items = f(widget);

                if let DisplayCommand::SolidColor(_, rect) = items[0].command {
                    if widget.get_size_request().1 != rect.height.ceil_to_px() {
                        widget.set_size_request(-1, rect.height.ceil_to_px())
                    }
                }

                for item in &items {
                    if match &item.command {
                        &DisplayCommand::SolidColor(_, rect)
                        | &DisplayCommand::Image(_, rect)
                        | &DisplayCommand::Text(_, rect, _, _, _) => {
                            let rect_y = rect.y.to_px();
                            let rect_height = rect.height.to_px();
                            let sy = max(rect_y, redraw_start_y as i32);
                            let ey = min(rect_y + rect_height, redraw_end_y as i32);
                            ey - sy > 0
                        }
                    } {
                        render_item(cairo_context, &mut pango_layout, &item.command);
                    }
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
                rect.x.to_f64_px(),
                rect.y.to_f64_px(),
                rect.width.to_f64_px(),
                rect.height.to_f64_px(),
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
            ctx.set_source_pixbuf(
                &pixbuf
                    .scale_simple(
                        rect.width.to_f64_px() as i32,
                        rect.height.to_f64_px() as i32,
                        InterpType::Hyper,
                    )
                    .unwrap(),
                rect.x.to_f64_px(),
                rect.y.to_f64_px(),
            );
            ctx.paint();
        }
        &DisplayCommand::Text(ref text, rect, ref color, ref decorations, ref font) => {
            FONT_DESC.with(|font_desc| {
                let mut font_desc = font_desc.borrow_mut();
                font_desc.set_size(pango::units_from_double(px2pt(font.size.to_f64_px())));
                font_desc.set_style(font.slant.to_pango_font_slant());
                font_desc.set_weight(font.weight.to_pango_font_weight());

                let attr_list = pango::AttrList::new();
                for decoration in decorations {
                    match decoration {
                        &TextDecoration::Underline => {
                            attr_list.insert(
                                pango::Attribute::new_underline(pango::Underline::Single).unwrap(),
                            );
                        }
                        &TextDecoration::Overline => unimplemented!(),
                        &TextDecoration::LineThrough => {
                            attr_list.insert(pango::Attribute::new_strikethrough(true).unwrap());
                        }
                        &TextDecoration::None => {}
                    }
                }

                pango_layout.set_text(text.as_str());
                pango_layout.set_attributes(Some(&attr_list));
                pango_layout.set_font_description(Some(&*font_desc));
            });

            ctx.set_source_rgba(
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
                color.a as f64 / 255.0,
            );
            ctx.move_to(rect.x.to_f64_px(), rect.y.to_f64_px());

            pangocairo::functions::show_layout(ctx, &pango_layout);
        }
    }
}

pub fn render<F: 'static>(f: F)
where
    F: Fn(&gtk::DrawingArea) -> DisplayList,
{
    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let window = RenderingWindow::new(800, 520, f);
    window.exit_on_close();

    gtk::main();
}
