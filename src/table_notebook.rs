use gtk::*;
use gio::prelude::*;
use std::env::args;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::collections::HashMap;
// use gtk_plots::conn_popover::{ConnPopover, TableDataSource};
use std::path::PathBuf;
// use sourceview::*;
use std::ffi::OsStr;
use gdk::ModifierType;
use tables::{ environment_source::EnvironmentSource, TableEnvironment, button::TableChooser};
use crate::table_widget::*;
//use gtk::builder::BuilderExtManual;
use gtk::prelude::*;
use nlearn::table::*;

#[derive(Clone)]
pub struct TableNotebook {
    pub nb : Notebook
}

impl TableNotebook {

    pub fn new(builder : &Builder) -> TableNotebook {
        let nb : Notebook =
            builder.get_object("tables_notebook").unwrap();
        TableNotebook{nb}
    }

    pub fn clear(&self) {
        for w in self.nb.get_children() {
            self.nb.remove(&w);
        }
    }

    pub fn get_page_index(&self) -> usize {
        self.nb.get_property_page() as usize
    }

    pub fn add_page(&self,
        icon : &str,
        label : Option<&str>,
        err_msg : Option<&str>,
        data : Option<Vec<Vec<String>>>
    ) {
        let img = Image::new_from_icon_name(
            Some(icon), IconSize::LargeToolbar
        );
        let mut table_w = TableWidget::new();
        self.nb.add(&(table_w.scroll_window));
        let box_label = Box::new(Orientation::Horizontal, 0);
        box_label.pack_start(&img, true, true, 0);
        box_label.pack_start(
            &Label::new(label), true, true, 0);
        self.nb.set_tab_label(&(table_w.scroll_window), Some(&box_label));
        box_label.show_all();
        //let npages = nb.get_children().len() as i32;
        //nb.set_property_page(npages-1);
        self.nb.show_all();
        if let Some(rows) = data {
            table_w.update_data(rows);
            table_w.show_data();
        }
        if let Some(msg) = err_msg {
            table_w.show_message(msg);
        }
        self.nb.next_page();
    }
}

