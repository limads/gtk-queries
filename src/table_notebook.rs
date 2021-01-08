use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::table_widget::*;
use gtk::prelude::*;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::table_popover::TablePopover;
use std::collections::HashMap;
use gdk_pixbuf::Pixbuf;

#[derive(Debug, Clone)]
pub enum TableSource {
    Command(String),
    File(String),
    Database(Option<String>, Option<String>)
}

const ICONS : [&'static str; 5] = [
    "grid-black.svg",
    "inner.svg",
    "left.svg",
    "right.svg",
    "full.svg"
];

#[derive(Clone)]
pub struct TableNotebook {
    pub nb : Notebook,
    pub tbls : Rc<RefCell<Vec<TableWidget>>>,
    icons : HashMap<&'static str, Pixbuf>,
    sources : Rc<RefCell<Vec<TableSource>>>
}

impl TableNotebook {

    pub fn new(builder : &Builder) -> TableNotebook {
        let nb : Notebook =
            builder.get_object("tables_notebook").unwrap();
        let tbls = Rc::new(RefCell::new(Vec::new()));
        let mut icons = HashMap::new();

        for icon in ICONS.iter() {
            let pix = Pixbuf::from_file_at_scale(&format!("assets/icons/{}", icon), 16, 16, true).unwrap();
            icons.insert(*icon, pix);
        }
    
        let sources = Rc::new(RefCell::new(Vec::new()));
        let tbl_nb = TableNotebook{nb, tbls, icons, sources};
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
        self.sources.borrow_mut().clear();
    }

    pub fn set_page_index(&self, page : usize) {
        self.nb.set_property_page(page as i32);
    }

    pub fn get_page_index(&self) -> usize {
        self.nb.get_property_page() as usize
    }

    // TODO unselect all columns on notebook page switch

    pub fn create_error_table(&self, msg : &str) {
        let mut table_w = TableWidget::new();
        self.nb.add(&(table_w.scroll_window));
        let img = Image::from_icon_name(Some("close-symbolic"), IconSize::Menu);
        let (label_bx, table_w) = self.create_table(&img, &Label::new(Some("Error")));
        table_w.show_message(msg);
        self.nb.set_tab_label(&(table_w.scroll_window), Some(&label_bx));
    }
    
    fn create_table(&self, img : &Image, lbl : &Label) -> (gtk::Box, TableWidget) {
        img.set_margin_start(6);
        img.set_margin_end(6);
        let mut table_w = TableWidget::new();
        let box_label = Box::new(Orientation::Horizontal, 0);
        box_label.pack_start(img, false, false, 0);
        box_label.pack_start(lbl, false, false, 0);
        self.nb.add(&(table_w.scroll_window));
        self.nb.next_page();
        if let Ok(mut tbls) = self.tbls.try_borrow_mut() {
            tbls.push(table_w.clone());
        } else {
            println!("Could not retrieve mutable reference to table widget");
        }
        (box_label, table_w)
    }
    
    pub fn create_data_table(
        &self,
        table_source : TableSource,
        rows : Vec<Vec<String>>,
        workspace : PlotWorkspace,
        table_popover : TablePopover
    ) {
        if rows.len() == 0 {
            println!("No rows to display");
            return;
        }
        let (icon, mut name) = match table_source.clone() {
            TableSource::Command(cmd) => (format!("bash-symbolic"), format!("Std. out ({})", cmd.clone())),
            TableSource::File(path) => (format!("folder-documents-symbolic"), path.clone()),
            TableSource::Database(name, rel) => match (name, rel) {
                (Some(name), Some(rel)) => (name.clone(), format!("{}.svg", rel)),
                (Some(name), None) => (format!("grid-black.svg"), name.clone()),
                _ => (format!("grid-black.svg"), format!("Unknown"))
            }
        };
        let (nrows, ncols) = (rows.len(), rows[0].len());
        name += &format!(" ({} x {})", nrows - 1, ncols);
        let img = match self.icons.get(&icon[..]) {
            Some(pxb) => Image::from_pixbuf(Some(&self.icons[&icon[..]])),
            None => Image::from_icon_name(Some(&icon), IconSize::Menu)
        };
        
        let (box_label, mut table_w) = self.create_table(&img, &Label::new(Some(&name)));
        
        let ev_bx = EventBox::new();
        ev_bx.add(&box_label);
        let tbl_ix = self.len();
        {
            let table_source = table_source.clone();
            ev_bx.connect_button_press_event(move |ev_box, ev| {
                if ev.get_button() == 3 {
                    match table_source {
                        TableSource::Database(_, _) => {
                            table_popover.set_copy_to();
                        },
                        _ => { 
                            table_popover.set_copy_from();
                        }
                    }
                    table_popover.show_at(&ev_box, tbl_ix);
                }
                glib::signal::Inhibit(false)
            });
        }
        box_label.show_all();
        self.nb.show_all();
        self.nb.set_tab_label(&(table_w.scroll_window), Some(&ev_bx));
        
        table_w.update_data(rows);
        table_w.show_data();

        // Left-click events
        {
            let mapping_popover = workspace.layout_toolbar.mapping_popover.clone();
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
            let mapping_popover = workspace.layout_toolbar.mapping_popover.clone();
            // let mapping_menus = workspace.mapping_menus.clone();
            let layout_toolbar = workspace.layout_toolbar.clone();
            // let plot_popover = workspace.plot_popover.clone();
            let sources = workspace.sources.clone(); 
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
                        // mapping_menus.clone(),
                        // &plot_popover,
                        sources.clone(),
                        &selected,
                        curr_tbl_ix
                    );
                }
                mapping_popover.show();
                glib::signal::Inhibit(false)
            }, 3);
        }
        
        self.sources.borrow_mut().push(table_source);
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
        for (_i, tbl) in self.tbls.borrow().iter().enumerate() {
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


