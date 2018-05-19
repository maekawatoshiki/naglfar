extern crate cairo;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate pango;
extern crate pangocairo;

use gtk::{Inhibit, ObjectExt, WidgetExt, traits::*};
use gtk::ContainerExt;

use glib::prelude::*; // or `use gtk::prelude::*;`
use glib;

use gdk::{ContextExt, Cursor, CursorType, Event, EventButton, EventMask, EventMotion, RGBA};
use gdk_pixbuf::{InterpType, PixbufExt};

use cairo::Context;
use pango::LayoutExt;

use std::{cell::RefCell, collections::HashMap};

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
    // HashMap<URL Fragment(id), y coordinate of the content>
    pub static URL_FRAGMENTS: RefCell<HashMap<String, f64>> = { RefCell::new(HashMap::with_capacity(8)) };
    pub static BUTTONS: RefCell<HashMap<usize, gtk::Button>> = { RefCell::new(HashMap::with_capacity(8)) };
    pub static SURFACE_CACHE: RefCell<Option<cairo::ImageSurface>> = { RefCell::new(None) };
);

static mut RESIZED: bool = false;

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
        window.override_background_color(
            gtk::StateFlags::from_bits(gtk::StateFlags::NORMAL.bits()).unwrap(),
            Some(&RGBA {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0,
            }),
        );

        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(width, height);

        let layout = gtk::Layout::new(None, None);

        let overlay = gtk::Overlay::new();
        {
            use gtk::OverlayExt;
            overlay.add_overlay(&drawing_area);
            overlay.set_child_index(&drawing_area, 0);
            overlay.add_overlay(&layout);
            overlay.set_child_index(&layout, 1);
        }

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let entry = gtk::Entry::new();
        vbox.pack_start(&entry, false, false, 0);

        entry
            .connect("activate", true, |args| {
                let entry = args[0]
                    .clone()
                    .downcast::<glib::Object>()
                    .unwrap()
                    .get()
                    .unwrap()
                    .downcast::<gtk::Entry>()
                    .unwrap();
                let vbox = entry.get_parent().unwrap().downcast::<gtk::Box>().unwrap();
                let scrolled_window = vbox.get_children()[1]
                    .clone()
                    .downcast::<gtk::ScrolledWindow>()
                    .unwrap();
                let viewport = scrolled_window
                    .get_child()
                    .unwrap()
                    .downcast::<gtk::Viewport>()
                    .unwrap();
                let overlay = viewport
                    .get_child()
                    .unwrap()
                    .downcast::<gtk::Overlay>()
                    .unwrap();
                let drawing_area = overlay.get_children()[0].clone();

                let url = entry.get_text().unwrap();
                println!("URL: {}", url);
                update_html_tree_and_stylesheet(url);
                ANKERS.with(|ankers| ankers.borrow_mut().clear());
                SURFACE_CACHE.with(|sc| *sc.borrow_mut() = None);
                drawing_area.queue_draw();
                None
            })
            .unwrap();

        let scrolled_window = gtk::ScrolledWindow::new(None, None);
        scrolled_window.add(&overlay);
        vbox.pack_start(&scrolled_window, true, true, 0);

        window.add(&vbox);
        overlay.add_events(
            EventMask::POINTER_MOTION_MASK.bits() as i32
                | EventMask::BUTTON_PRESS_MASK.bits() as i32,
        );

        overlay
            .connect("motion-notify-event", false, |args| {
                use gdk::WindowExt;
                let overlay = args[0]
                    .clone()
                    .downcast::<gtk::Overlay>()
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
                    let window = overlay.get_window().unwrap();
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

        overlay
            .connect("button-press-event", false, |args| {
                let overlay = args[0]
                    .clone()
                    .downcast::<gtk::Overlay>()
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
                                overlay.get_children()[0].queue_draw(); // [0] is DrawingArea
                            }
                            &AnkerKind::URLFragment(ref id) => {
                                URL_FRAGMENTS.with(|ufs| {
                                    if let Some(content_y) = ufs.borrow().get(id) {
                                        let mut adjustment = overlay
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
                        ankers.clear();
                        SURFACE_CACHE.with(|sc| *sc.borrow_mut() = None);
                    }
                });
                Some(true.to_value())
            })
            .unwrap();

        window.connect_configure_event(|_, _| {
            unsafe {
                RESIZED = true;
            }
            false
        });

        let instance = RenderingWindow {
            window: window,
            drawing_area: drawing_area,
        };

        instance
            .drawing_area
            .connect_draw(move |widget, cairo_context| {
                // println!("here");

                // let overlay = widget
                //     .get_parent()
                //     .unwrap()
                //     .downcast::<gtk::Overlay>()
                //     .unwrap();
                // let layout = &overlay.get_children()[1]
                //     .clone()
                //     .downcast::<gtk::Layout>()
                //     .unwrap(); // [1] is Layout

                let surface = SURFACE_CACHE.with(|sc| {
                    if let Some(ref surface) = *sc.borrow_mut() {
                        unsafe {
                            if RESIZED {
                                RESIZED = false;
                            } else {
                                return surface.clone();
                            }
                        }
                    }

                    let pango_ctx = widget.create_pango_context().unwrap();
                    let mut pango_layout = pango::Layout::new(&pango_ctx);

                    let items = f(widget);
                    let content_rect =
                        if let DisplayCommand::SolidColor(_, content_rect) = items[0].command {
                            content_rect
                        } else {
                            unreachable!()
                        };

                    widget
                        .get_parent()
                        .unwrap()
                        .downcast::<gtk::Overlay>()
                        .unwrap()
                        .set_size_request(-1, content_rect.height.ceil_to_px());
                    widget.set_size_request(-1, content_rect.height.ceil_to_px());

                    let surface = cairo::ImageSurface::create(
                        cairo::Format::ARgb32,
                        content_rect.width.to_px(),
                        content_rect.height.to_px(),
                    ).unwrap();
                    let ctx = cairo::Context::new(&surface);
                    for item in &items {
                        render_item(&ctx, &mut pango_layout, /* layout, */ &item.command);
                    }

                    // let radial = cairo::LinearGradient::new(0.0, 0.0, 0.0, 200.0);
                    // use cairo::Gradient;
                    // radial.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.5);
                    // radial.add_color_stop_rgba(0.4, 0.0, 0.0, 0.0, 0.0);
                    // ctx.mask(&radial);

                    *sc.borrow_mut() = Some(surface.clone());
                    surface
                });

                let (_, redraw_start_y, redraw_end_x, redraw_end_y) = cairo_context.clip_extents();

                cairo_context.set_source_surface(&surface, 0.0, 0.0);
                cairo_context.rectangle(
                    0.0,
                    redraw_start_y,
                    redraw_end_x,
                    redraw_end_y - redraw_start_y,
                );
                cairo_context.fill();

                // layout.show_all();

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

fn render_item(
    ctx: &Context,
    pango_layout: &mut pango::Layout,
    // layout: &gtk::Layout,
    item: &DisplayCommand,
) {
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
        &DisplayCommand::Button(ref _btn, _rect) => {
            // use gtk::LayoutExt;
            // layout.put(btn, rect.x.ceil_to_px(), rect.y.ceil_to_px());
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
