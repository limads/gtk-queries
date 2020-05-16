use gtk::*;
use gio::prelude::*;
use std::fs::File;
use std::io::Read;
use gtkplotview::plot_view::{PlotView, UpdateContent};
use gtkplotview::PlotArea;
use std::path::PathBuf;
use std::io::Write;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;

pub fn build_save_widgets(
    builder : &Builder,
    pl_view : Rc<RefCell<PlotView>>
) {
    /*let xml_save_dialog : FileChooserDialog =
    builder.get_object("xml_save_dialog").unwrap();
    {
        let rc_view = pl_view.clone();
        xml_save_dialog.connect_response(move |dialog, resp|{
            // println!("{:?}", resp);
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        if let Ok(pl) = rc_view.try_borrow() {
                            if let Ok(mut f) = File::create(path) {
                                let content = pl.plot_area.get_layout_as_text();
                                let _ = f.write_all(&content.into_bytes());
                            }
                        }
                    }
                },
                _ => { }
            }
        });
    }
    let save_layout_btn : Button =
        builder.get_object("save_layout_btn").unwrap();
    let xml_save_rc = xml_save_dialog.clone();
    save_layout_btn.connect_clicked(move |_| {
        xml_save_rc.run();
        xml_save_rc.hide();
    });*/

    let save_img_dialog : FileChooserDialog =
        builder.get_object("save_img_dialog").unwrap();

    {
        let save_img_dialog = save_img_dialog.clone();
        let save_image_btn : Button =
            builder.get_object("save_image_btn").unwrap();
        save_image_btn.connect_clicked(move |btn| {
            save_img_dialog.run();
            save_img_dialog.hide();
        });
    }

    save_img_dialog.connect_response(move |dialog, resp|{
        match resp {
            ResponseType::Other(1) => {
                if let Some(path) = dialog.get_filename() {
                    if let Ok(mut pl) = pl_view.try_borrow_mut() {
                        if let Some(p) = path.to_str() {
                            pl.plot_group.draw_to_file(p, 800, 600);
                        } else {
                            println!("Could not retrieve path as str");
                        }
                    } else {
                        println!("Could not retrieve reference to pl_view when saving image");
                    }
                } else {
                    println!("Invalid path for image");
                }
            },
            _ => { }
        }
    });
}


