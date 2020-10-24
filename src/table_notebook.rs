use gtk::*;
use std::rc::Rc;
use std::cell::{RefCell};
use crate::table_widget::*;
use gtk::prelude::*;
use crate::plots::layout_window::*;
use crate::plots::layout_toolbar::LayoutToolbar;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::table_popover::TablePopover;

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
                println!("Unselect now");
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

    pub fn set_page_index(&self, page : usize) {
        self.nb.set_property_page(page as i32);
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
        workspace : PlotWorkspace,
        table_popover : TablePopover
    ) {
        let img = Image::from_icon_name(
            Some(icon), IconSize::LargeToolbar
        );
        let mut table_w = TableWidget::new();
        self.nb.add(&(table_w.scroll_window));
        let box_label = Box::new(Orientation::Horizontal, 0);
        box_label.pack_start(&img, true, true, 0);
        box_label.pack_start(&Label::new(label), true, true, 0);
        let ev_bx = EventBox::new();
        ev_bx.add(&box_label);
        ev_bx.connect_button_press_event(move |ev_box, ev| {
            if ev.get_button() == 3 {
                println!("Right click at current table");
                //let rel_to_self = table_popover.popover.get_relative_to() == Some(ev_box.into());
                //if !table_popover.popover.is_visible() || !rel_to_self {
                //    if !rel_to_self {
                table_popover.popover.hide();
                table_popover.popover.set_relative_to(Some(ev_box));
                //    }
                table_popover.popover.show();
                // }
            }
            glib::signal::Inhibit(false)
        });
        self.nb.set_tab_label(&(table_w.scroll_window), Some(&ev_bx));
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
                    println!("Selected columns (left click): {:?}", selected);
                    // mapping_popover.set_relative_to(Some(ev_bx));
                    mapping_popover.hide();
                    glib::signal::Inhibit(false)
                }, 1);
            }

            // Right-click events
            {
                let mapping_popover = mapping_popover.clone();
                let mapping_menus = workspace.mapping_menus.clone();
                let layout_toolbar = workspace.layout_toolbar.clone();
                let plot_popover = workspace.plot_popover.clone();

                // Assume the table is static at the same position through all calls of
                // set_selected_action. This is valid because for now the whole table
                // environment is cleared when there are any query changes. Note we use
                // len here and not len-1 becaues the element will effectively be added
                // to the tbls vector only at the end of this method.
                let curr_tbl_ix = self.len();

                table_w.set_selected_action(move |ev_bx, _ev, selected, curr| {
                    println!("Selected column set via left click: {:?}", selected);
                    println!("Currently selected column via right click: {}", curr);
                    if selected.iter().find(|s| **s == curr).is_some() {
                        mapping_popover.set_relative_to(Some(ev_bx));
                        layout_toolbar.update_mapping_status(
                            mapping_menus.clone(),
                            &plot_popover,
                            &selected,
                            curr_tbl_ix
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

    /// Returns selected columns, as a pair of (table, selected columns).
    pub fn selected_table_and_cols(&self) -> Option<(usize, Vec<usize>)> {
        if let Ok(tbls) = self.tbls.try_borrow() {
            for (i, t) in tbls.iter().enumerate() {
                let sel = t.selected_cols();
                if sel.len() > 0 {
                    return Some((i, sel));
                }
            }
        } else {
            println!("Unable to retrieve reference to tables");
        }
        None
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

    pub fn len(&self) -> usize {
        self.tbls.borrow().len()
    }

}


