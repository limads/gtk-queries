use gtk::*;
// use gio::prelude::*;
// use std::env::args;
use std::rc::Rc;
use std::cell::{RefCell /*, RefMut*/ };
// use std::fs::File;
// use std::io::Write;
// use std::io::Read;
// use std::collections::HashMap;
// use gtk_plots::conn_popover::{ConnPopover, TableDataSource};
// use std::path::PathBuf;
// use sourceview::*;
// use std::ffi::OsStr;
// use gdk::ModifierType;
// use crate::tables::{ source::EnvironmentSource, environment::TableEnvironment, /*button::TableChooser*/ };
use crate::table_widget::*;
//use gtk::builder::BuilderExtManual;
use gtk::prelude::*;
// use crate::tables::table::*;
// use crate::functions::function_search::*;
use crate::plots::layout_menu::*;
use crate::plots::layout_toolbar::LayoutToolbar;

#[derive(Clone)]
pub struct TableNotebook {
    pub nb : Notebook,
    pub tbls : Rc<RefCell<Vec<TableWidget>>>
}

impl TableNotebook {

    pub fn new(builder : &Builder) -> TableNotebook {
        let nb : Notebook =
            builder.get_object("tables_notebook").unwrap();
        let tbls = Rc::new(RefCell::new(Vec::new()));
        let tbl_nb = TableNotebook{nb, tbls};
        {
            let tbl_nb = tbl_nb.clone();
            tbl_nb.nb.clone().connect_change_current_page(move |_, _| {
                tbl_nb.unselect_all_tables();
                true
            });
        }
        tbl_nb
    }

    pub fn clear(&self) {
        for w in self.nb.get_children() {
            self.nb.remove(&w);
        }
        if let Ok(mut tbls) = self.tbls.try_borrow_mut() {
            tbls.clear();
        } else {
            println!("Unable to get mutable reference to table vector");
        }
    }

    pub fn get_page_index(&self) -> usize {
        self.nb.get_property_page() as usize
    }

    // TODO unselect all columns on notebook page switch

    pub fn add_page(
        &self,
        icon : &str,
        label : Option<&str>,
        err_msg : Option<&str>,
        data : Option<Vec<Vec<String>>>,
        mapping_popover : Popover,
        sidebar : PlotSidebar,
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
        self.nb.show_all();
        if let Some(rows) = data {
            table_w.update_data(rows);
            table_w.show_data();

            // Left-click events
            {
                let mapping_popover = mapping_popover.clone();
                // let mapping_menus = sidebar.mapping_menus.clone();
                // let layout_toolbar = sidebar.layout_toolbar.clone();
                // let plot_popover = sidebar.plot_popover.clone();
                table_w.set_selected_action(move |ev_bx, _ev, selected, curr| {
                    println!("Selected: {:?}", selected);
                    // mapping_popover.set_relative_to(Some(ev_bx));
                    mapping_popover.hide();
                    glib::signal::Inhibit(false)
                }, 1);
            }

            // Right-click events
            {
                let mapping_popover = mapping_popover.clone();
                let mapping_menus = sidebar.mapping_menus.clone();
                let layout_toolbar = sidebar.layout_toolbar.clone();
                let plot_popover = sidebar.plot_popover.clone();
                table_w.set_selected_action(move |ev_bx, _ev, selected, curr| {
                    println!("All selected: {:?}", selected);
                    println!("Currently selected: {}", curr);
                    if selected.iter().find(|s| **s == curr).is_some() {
                        mapping_popover.set_relative_to(Some(ev_bx));
                        layout_toolbar.set_add_or_edit_mapping_sensitive(
                            &mapping_menus,
                            &plot_popover,
                            &selected,
                        );
                    }
                    mapping_popover.show();
                    glib::signal::Inhibit(false)
                }, 3);
            }
        }
        if let Some(msg) = err_msg {
            table_w.show_message(msg);
        }
        self.nb.next_page();
        if let Ok(mut tbls) = self.tbls.try_borrow_mut() {
            tbls.push(table_w);
        } else {
            println!("Could not retrieve mutable reference to table widget");
        }
    }

    /// Get selected cols across whole session. Indices are set relative to
    /// the first column of the first table, and increase up to the last
    /// column of the last table.
    pub fn full_selected_cols(&self) -> Vec<usize> {
        let mut cols = Vec::new();
        let mut base_ix = 0;
        if let Ok(tbls) = self.tbls.try_borrow() {
            for tbl in tbls.iter() {
                let mut selected = tbl.selected_cols();
                // println!("Selected at table: {:?}", selected);
                selected.iter_mut().for_each(|ix| *ix += base_ix);
                // println!("Full selected: {:?}", selected);
                cols.extend(selected);
                base_ix += tbl.dimensions().1;
            }
        }
        cols
    }

    /// Returns selected columns, as a pair of (table, selected column).
    pub fn selected_table_and_cols(&self) -> Vec<(usize, usize)> {
        let mut sel = Vec::new();
        if let Ok(tbls) = self.tbls.try_borrow() {
            for (i, t) in tbls.iter().enumerate() {
                for s in t.selected_cols() {
                    sel.push((i, s));
                }
            }
        } else {
            println!("Unable to retrieve reference to tables");
        }
        sel
    }

    /// Get selected cols at the current selected page. Indices are relative to
    /// the first column of the selected table.
    pub fn selected_cols(&self) -> Vec<usize> {
        let ix = self.get_page_index();
        if let Ok(tbls) = self.tbls.try_borrow() {
            if let Some(tbl) = tbls.get(ix) {
                tbl.selected_cols()
            } else {
                Vec::new()
            }
        } else {
            println!("Could not retrieve mutable reference to table widget");
            Vec::new()
        }
    }

    pub fn unselect_all_tables(&self) {
        if let Ok(tbls) = self.tbls.try_borrow() {
            for t in tbls.iter() {
                t.unselect_all();
            }
        } else {
            println!("Could not get borrow over tables");
        }
    }

    pub fn unselect_at_table(&self) {
        let ix = self.get_page_index();
        if let Ok(tbls) = self.tbls.try_borrow() {
            if let Some(tbl) = tbls.get(ix) {
                tbl.unselect_all();
            }
        } else {
            println!("Unable to retrieve reference to tables");
        }
    }

    pub fn set_selected_cols(&self, global_ixs : &[usize]) {
        let mut base_ix : usize = 0;
        for (i, tbl) in self.tbls.borrow().iter().enumerate() {
            let ncols = tbl.dimensions().1;
            let curr_ixs : Vec<usize> = global_ixs.iter()
                .filter(|ix| **ix >= base_ix && **ix < base_ix + ncols)
                .map(|ix| ix - base_ix).collect();
            if curr_ixs.len() > 0 {
                tbl.set_selected(&curr_ixs[..]);
            }
            base_ix += ncols;
        }
    }

    pub fn expose_table(&self, ix : usize) -> Option<TableWidget> {
        self.tbls.try_borrow().ok()?
            .iter()
            .skip(ix)
            .next()
            .cloned()
    }

}


