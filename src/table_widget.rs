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
use nlearn::table::*;
use crate::utils;

pub struct TableWidget {
    grid : Grid,
    data : Vec<Vec<String>>,
    pub scroll_window : ScrolledWindow,
    box_container : Box,
    msg : Label,
    parent_ctx : StyleContext,
    provider : CssProvider
}

impl TableWidget {

    pub fn new() -> TableWidget {
        let grid = Grid::new();
        let data : Vec<Vec<String>> = Vec::new();
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
        TableWidget{grid, data, scroll_window,
            box_container, msg, parent_ctx, provider}
    }

    fn create_data_cell(&self,
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
        self.update_table_dimensions(
            nrows as i32, ncols as i32);
        for (i,row) in data.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                let cell = self.create_data_cell(
                    &col[..], i, j, nrows, ncols);
                self.grid.add(&cell);
                self.grid.set_cell_left_attach(&cell, j as i32);
                self.grid.set_cell_top_attach(&cell, i as i32);
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
}

