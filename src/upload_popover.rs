use gtk::*;
// use gio::prelude::*;
// use std::env::{self, args};
use std::rc::Rc;
use std::cell::{RefCell /*, RefMut*/ };
use std::fs::File;
// use std::io::Write;
use std::io::Read;
// use std::collections::HashMap;
// use gtk_plots::conn_popover::{ConnPopover, TableDataSource};
// use std::path::PathBuf;
// use sourceview::*;
// use std::ffi::OsStr;
// use gdk::ModifierType;
// use gdk::{self, /*enums::key*/ };
// use crate::tables::{self, source::EnvironmentSource, environment::TableEnvironment, sql::SqlListener};
// use crate::conn_popover::*;
// use sourceview::*;
// use std::boxed;
// use std::process::Command;
use gtk::prelude::*;
use crate::{ /*utils, table_widget::TableWidget,*/ table_notebook::TableNotebook };
// use crate::tables::table::Table;
// use crate::status_stack::*;
// use crate::sql_popover::*;
// use crate::functions::function_search::*;
// use crate::functions::num_function::*;
// use gdk::prelude::*;
// use crate::plots::plotview::plot_view::PlotView;
//use gtk_plots::save_widgets;
// use crate::plots::layout_menu::PlotSidebar;

#[derive(Clone)]
pub struct UploadPopover {
    toggle : ToggleButton,
    popover : Popover,
    chooser : FileChooserButton,
    switch : Switch,

    // Update from columns indexed by the first element; using SQL template from second element.
    update_info : Rc<RefCell<(Vec<usize>, String)>>
}

impl UploadPopover {

    pub fn new(builder : Builder, tbl_nb : TableNotebook) -> Self {
        let toggle : ToggleButton = builder.get_object("upload_toggle").unwrap();
        let popover : Popover = builder.get_object("upload_popover").unwrap();
        let chooser : FileChooserButton = builder.get_object("sql_upload_chooser").unwrap();
        let switch : Switch = builder.get_object("sql_upload_switch").unwrap();
        {
            let popover = popover.clone();
            toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    popover.show();
                } else {
                    popover.hide();
                }
            });
        }
        let update_info = Rc::new(RefCell::new((Vec::new(), String::new())));
        {
            let update_info = update_info.clone();
            chooser.connect_file_set(move |chooser| {
                if let Some(path) = chooser.get_filename() {
                    if let Ok(mut f) = File::open(path) {
                        if let Ok(mut info) = update_info.try_borrow_mut() {
                            if let Err(e) = f.read_to_string(&mut info.1) {
                                println!("{}", e);
                            }
                        } else {
                            println!("Could not retrieve mutable reference to sql upload file");
                        }
                    } else {
                        println!("Error opening file");
                    }
                }
            });
        }

        {
            let update_info = update_info.clone();
            switch.connect_state_set(move |_switch, _state| {
                let selected = tbl_nb.full_selected_cols();
                if let Ok(mut info) = update_info.try_borrow_mut() {
                    info.0.clear();
                    info.0.extend(selected);
                } else {
                    println!("Unable to get mutable reference to update info");
                }
                glib::signal::Inhibit(true)
            });
        }

        Self{ toggle, popover, chooser, switch, update_info }
    }

}

