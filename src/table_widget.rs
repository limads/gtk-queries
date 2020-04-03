use gtk::*;
use gtk::prelude::*;
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
use crate::tables::{ source::EnvironmentSource, environment::TableEnvironment};
use crate::tables::table::*;
use crate::utils;
use gdk::prelude::*;
use gdk::{Cursor, CursorType};

#[derive(Clone)]
pub struct TableWidget {
    grid : Grid,
    // data : Rc<RefCell<Vec<Vec<String>>>>,
    pub scroll_window : ScrolledWindow,
    box_container : Box,
    msg : Label,
    parent_ctx : StyleContext,
    provider : CssProvider,
    nrows : usize,
    ncols : usize,
    //tbl : Table,
    selected : Rc<RefCell<Vec<(String, usize, bool)>>>
}

impl TableWidget {

    pub fn new_from_table(tbl : &Table) -> Self {
        let mut tbl_wid = Self::new();
        let data = tbl.text_rows();
        tbl_wid.update_data(data);
        tbl_wid
    }

    pub fn new() -> TableWidget {
        let grid = Grid::new();
        // let data = Rc::new(RefCell::new(Vec::new()));
        let message = Label::new(None);
        let provider = utils::provider_from_path("tables.css")
            .expect("Unable to load tables CSS");
        let parent_ctx = grid.get_style_context();
        parent_ctx.add_provider(&provider,800);
        let msg = Label::new(None);
        let box_container = Box::new(Orientation::Vertical, 0);
        box_container.pack_start(&grid, true, true, 0);
        box_container.pack_start(&msg, true, true, 0);
        let scroll_window = ScrolledWindow::new(
            Some(&Adjustment::new(0.0, 0.0, 100.0, 10.0, 10.0, 100.0)),
            Some(&Adjustment::new(0.0, 0.0, 100.0, 10.0, 10.0, 100.0))
        );
        scroll_window.add(&box_container);
        scroll_window.show_all();
        let selected = Rc::new(RefCell::new(Vec::new()));
        //let tbl = Table::new_empty(None);
        TableWidget{grid, /*data,*/ scroll_window,
            box_container, msg, parent_ctx, provider, selected, nrows : 0, ncols : 0, /*tbl*/ }
    }

    pub fn parent(&self) -> ScrolledWindow {
        self.scroll_window.clone()
    }

    fn create_header_cell(
        &self,
        data : &str,
        row : usize,
        col : usize,
        nrows : usize,
        ncols : usize
    ) -> gtk::EventBox {
        let label = self.create_data_cell(data, row, col, nrows, ncols);
        let ev_box = gtk::EventBox::new();
        //ev_box.set_above_child(true);
        //ev_box.set_visible_window(true);
        ev_box.add(&label);
        if let Ok(mut sel) = self.selected.try_borrow_mut() {
            sel.push((data.to_string(), col, false));
        }
        ev_box
    }

    fn switch_to(
        grid : Grid,
        col : &mut (String, usize, bool),
        ncols : usize,
        selected : bool
    ) {
        Self::set_selected_style(grid.clone(), ncols, col.1, selected);
        *col = (col.0.clone(), col.1, selected);
    }

    fn switch_selected(grid : Grid, cols : &mut [(String, usize, bool)], pos : usize) {
        let ncols = cols.len();
        if let Some(col) = cols.get_mut(pos) {
            if col.2 == true {
                Self::switch_to(grid.clone(), col, ncols, false);
            } else {
                Self::switch_to(grid.clone(), col, ncols, true);
            }
        } else {
            println!("Invalid column index")
        }
    }

    fn set_selected_style(grid : Grid, ncols : usize, col : usize, selected : bool) {
        for wid in grid.get_children().iter().skip(ncols - col - 1).step_by(ncols) {
            let wid = if let Ok(ev) = wid.clone().downcast::<EventBox>() {
                ev.get_child().unwrap()
            } else {
                wid.clone()
            };
            let ctx = wid.get_style_context();
            if selected {
                if !ctx.has_class("selected") {
                    ctx.add_class("selected");
                }
            } else {
                if ctx.has_class("selected") {
                    ctx.remove_class("selected");
                }
            }
        }
    }

    fn create_data_cell(
        &self,
        data : &str,
        row : usize,
        col : usize,
        nrows : usize,
        ncols : usize
    ) -> Label {
        let label = Label::new(Some(data));
        label.set_hexpand(true);
        let ctx = label.get_style_context();
        ctx.set_parent(Some(&(self.parent_ctx)));
        ctx.add_provider(&(self.provider),800); // PROVIDER_CONTEXT_USER
        ctx.add_class("table-cell");

        // Add this only when all columns have a title, maybe on a set_header method.
        if row == 0 {
            ctx.add_class("first-row");
        }
        if row == nrows - 1 {
            ctx.add_class("last-row");
        }
        if col % 2 != 0 {
            ctx.add_class("odd-col");
        } else {
            ctx.add_class("even-col");
        }
        if col == ncols-1 {
            ctx.add_class("last-col");
        }
        if (row + 1) % 2 != 0 {
            ctx.add_class("odd-row");
        }

        label
    }

    pub fn selected_cols(&self) -> Vec<usize> {
        if let Ok(sel) = self.selected.try_borrow() {
            sel.iter().filter(|s| s.2 == true ).map(|s| s.1 ).collect()
        } else {
            println!("Selected is borrowed");
            Vec::new()
        }
    }

    pub fn unselected_cols(&self) -> Vec<usize> {
        let n = if let Ok(sel) = self.selected.try_borrow() {
            sel.len()
        } else {
            println!("Selected is borrowed");
            return Vec::new();
        };
        let selected = self.selected_cols();
        let mut unselected = Vec::new();
        for i in 0..n {
            if !selected.iter().any(|s| i == *s) {
                unselected.push(i);
            }
        }
        unselected
    }

    pub fn unselect_all(&self) {
        let selected = self.selected_cols();
        if let Ok(mut sel) = self.selected.try_borrow_mut() {
            for s in selected {
                Self::switch_selected(self.grid.clone(), &mut sel, s);
            }
        } else {
            println!("Could not retrieve mutable reference to selected");
        }
    }

    fn switch_all(grid : Grid, cols : &mut [(String, usize, bool)]) {
        let switch_to = !cols.iter().any(|c| c.2);
        let n = cols.len();
        for i in 0..n {
            Self::switch_to(grid.clone(), &mut cols[i], n, switch_to);
        }
    }

    pub fn set_selected_action<F>(&self, f : F)
        where
            F : Clone,
            for<'r,'s> F : Fn(&'r EventBox, &'s gdk::EventButton, Vec<usize>)->Inhibit+'static
    {
        for wid in self.grid.get_children().iter().skip(self.ncols * self.nrows - self.ncols) {
            if let Ok(ev_box) = wid.clone().downcast::<EventBox>() {
                let selected = self.selected.clone();
                let f = f.clone();
                ev_box.connect_button_press_event(move |ev_box, ev| {
                    if let Ok(sel) = selected.try_borrow() {
                        let sel_ix : Vec<_> = sel.iter().filter(|c| c.2).map(|c| c.1).collect();
                        f(ev_box, ev, sel_ix);
                    } else {
                        println!("Unable to retrieve reference to selected vector");
                    }
                    glib::signal::Inhibit(false)
                });
            } else {
                println!("Could not convert widget to event box");
            }
        }
    }

    pub fn update_data(&mut self, data : Vec<Vec<String>>) {
        self.clear_table();
        if data.is_empty() {
            return;
        }
        if data.len() >= 1 && data[0].is_empty() {
            return;
        }
        let nrows = data.len();
        let ncols = data[0].len();
        self.nrows = nrows;
        self.ncols = ncols;
        self.update_table_dimensions(
            nrows as i32, ncols as i32);
        for (i,row) in data.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if i == 0 {
                    let ev_box = self.create_header_cell(
                        &col[..], i, j, nrows, ncols);
                    self.grid.add(&ev_box);
                    ev_box.realize();
                    if let Some(win) = ev_box.get_window() {
                        let disp = win.get_display();
                        let cur = Cursor::new_for_display(&disp, CursorType::Hand2);
                        win.set_cursor(Some(&cur));
                        let mut ev_mask = ev_box.get_events();
                        ev_mask.set(gdk::EventMask::BUTTON_PRESS_MASK, true);
                        ev_mask.set(gdk::EventMask::POINTER_MOTION_MASK, true); //GDK_2BUTTON_PRESS
                        {
                            let selected = self.selected.clone();
                            let grid = self.grid.clone();
                            ev_box.connect_button_press_event(move |bx, ev| {
                                if let Some(child) = bx.get_children().get(0) {
                                    let label : Label = child.clone().downcast().unwrap();
                                    if let Some(txt) = label.get_text().map(|t| t.to_string()) {
                                        if let Ok(mut sel) = selected.try_borrow_mut() {
                                            if ev.get_click_count() == Some(1) {
                                                if let Some(pos) = sel.iter_mut().position(|c| &c.0[..] == &txt[..] ) {
                                                    Self::switch_selected(grid.clone(), &mut sel[..], pos);
                                                } else {
                                                    println!("Invalid column name");
                                                }
                                            } else {
                                                Self::switch_all(grid.clone(), &mut sel[..]);
                                            }
                                        } else {
                                           println!("Selected vector mutably borrowed");
                                           return glib::signal::Inhibit(false);
                                        };
                                    } else {
                                        println!("Label does not have text");
                                    }
                                } else {
                                    println!("Label not present inside event box");
                                }
                                glib::signal::Inhibit(false)
                            });
                        }
                        /*ev_mask.set(gdk::EventMask::TOUCH_MASK, true);
                        ev_mask.set(gdk::EventMask::BUTTON1_MOTION_MASK, true);
                        ev_mask.set(gdk::EventMask::BUTTON2_MOTION_MASK, true);
                        ev_mask.set(gdk::EventMask::BUTTON3_MOTION_MASK, true);
                        ev_box.set_events(ev_mask);
                        ev_box.connect_drag_begin(move |bx, ctx| {
                             if let Some(child) = bx.get_children().get(0) {
                                let label : Label = child.clone().downcast().unwrap();
                                if let Some(txt) = label.get_text() {
                                    println!("{:?}", txt);
                                }
                            }
                        });

                        ev_box.connect_drag_end(move |bx, ctx| {
                             if let Some(child) = bx.get_children().get(0) {
                                let label : Label = child.clone().downcast().unwrap();
                                if let Some(txt) = label.get_text() {
                                    println!("{:?}", txt);
                                }
                            }
                        });
                        ev_box.connect_drag_motion(move |bx, ctx, _, _, _| {
                             if let Some(child) = bx.get_children().get(0) {
                                let label : Label = child.clone().downcast().unwrap();
                                if let Some(txt) = label.get_text() {
                                    println!("{:?}", txt);
                                }
                            }
                            glib::signal::Inhibit(false)
                        });*/
                    }
                    self.grid.set_cell_left_attach(&ev_box, j as i32);
                    self.grid.set_cell_top_attach(&ev_box, i as i32);
                } else {
                    let cell = self.create_data_cell(
                        &col[..], i, j, nrows, ncols);
                    self.grid.add(&cell);
                    self.grid.set_cell_left_attach(&cell, j as i32);
                    self.grid.set_cell_top_attach(&cell, i as i32);
                }
            }
        }
        self.grid.show_all();
        self.grid.queue_draw();
    }

    fn clear_table(&self,) {
        while self.grid.get_children().len() > 0 {
            self.grid.remove_row(0);
        }
    }

    fn update_table_dimensions(&self, nrows : i32, ncols : i32) {

        for c in 0..ncols {
            self.grid.insert_column(c);
        }

        for r in 0..nrows {
            self.grid.insert_row(r);
        }

    }

    pub fn show_message(&self, msg : &str) {
        self.grid.hide();
        self.msg.set_text(msg);
        self.msg.show();
    }

    pub fn show_data(&self) {
        self.msg.set_text("");
        self.msg.hide();
        self.grid.show();
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.nrows, self.ncols)
    }

}

