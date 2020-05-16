use crate::tables::sql::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::*;
use gtk::prelude::*;
//use postgres::{Connection, TlsMode};
use std::collections::HashMap;
use crate::tables::environment::TableEnvironment;
use crate::tables::sql::{SqlListener};
use crate::tables::source::EnvironmentSource;
use gtk::prelude::*;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use crate::tables::table::*;
use crate::{utils, table_widget::TableWidget, table_notebook::TableNotebook };

#[derive(Clone)]
pub enum Status {
    Disconnected,

    /// There is a connection to the database and no queries were
    /// executed yet.
    Connected,
    ConnectionErr(String),

    /// There is a connection to the database AND all last executed
    /// non-select statements were successful.
    StatementExecuted(String),

    /// Last executed query or other statement were unsucessful.
    SqlErr(String),

    /// There is a connection to the database AND all last executed
    /// queries were succesful.
    Ok
}

impl Status {

    pub fn index(&self) -> usize {
        match self {
            Status::Disconnected => 0,
            Status::Connected => 1,
            Status::ConnectionErr(_) => 2,
            Status::StatementExecuted(_) => 3,
            Status::SqlErr(_) => 4,
            Status::Ok => 0
        }
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            Status::ConnectionErr(s) => Some(&s),
            Status::StatementExecuted(s) => Some(&s),
            Status::SqlErr(s) => Some(&s),
            _ => None
        }
    }

}

#[derive(Clone)]
pub struct StatusStack {
    parent_stack : Stack,
    alt_wid : Widget,
    status_boxes : Vec<Box>,
    status_stack : Stack,
    status : Rc<RefCell<Status>>,
    stmt_label : Label,
    sql_err_label : Label,
    conn_err_label : Label
}

impl StatusStack {

    pub fn get_status(&self) -> Option<Status> {
        if let Ok(status) = self.status.try_borrow() {
            Some(status.clone())
        } else {
            println!("Unable to retrieve current status");
            None
        }
    }

    pub fn new(builder : Builder, parent_stack : Stack, alt_wid : Widget) -> Self {
        //let path = utils::glade_path("status-stack.glade").expect("Failed to load glade file");
        //let builder = Builder::new_from_file(path);
        let status_stack : Stack = builder.get_object("status_stack").unwrap();
        parent_stack.add_named(&status_stack, "status");
        parent_stack.set_visible_child_name("status");
        let stmt_label : Label = builder.get_object("stmt_label").unwrap();
        let sql_err_label : Label = builder.get_object("sql_err_label").unwrap();
        let conn_err_label : Label = builder.get_object("conn_err_label").unwrap();
        let status = Rc::new(RefCell::new(Status::Disconnected));
        let mut status_boxes = Vec::new();
        status_boxes.push(builder.get_object::<Box>("disconnected_box").unwrap());
        status_boxes.push(builder.get_object::<Box>("connected_box").unwrap());
        status_boxes.push(builder.get_object::<Box>("conn_err_box").unwrap());
        status_boxes.push(builder.get_object::<Box>("stmt_update_box").unwrap());
        status_boxes.push(builder.get_object::<Box>("sql_err_box").unwrap());
        Self{
            parent_stack,
            status_stack,
            status,
            stmt_label,
            sql_err_label,
            conn_err_label,
            alt_wid,
            status_boxes
        }
    }

    /// Show the current status, hiding the alt widget if status is a successful one
    pub fn show_curr_status(&self) {
        let status = if let Ok(status) = self.status.try_borrow() {
            status.clone()
        } else {
            return;
        };
        self.update(status);
        self.parent_stack.set_visible_child(&self.status_stack);
         //,
        //match status {
        /*    Status::Ok =>
            status => self.update(status)
        }*/
    }

    // Show alt only if status is ok. Show connected if connection is online; and
    // other status otherwise.
    pub fn try_show_alt_or_connected(&self) {
        if let Ok(status) = self.status.try_borrow() {
            match *status {
                Status::Ok => self.parent_stack.set_visible_child(&self.alt_wid),
                _ => self.parent_stack.set_visible_child_name("status"),
            }
        } else {
            println!("Could not borrow status");
        }
    }

    /// Show alt widget if status is any successful one (Ok|Connected);
    /// do nothing otherwise.
    pub fn try_show_alt(&self) {
        if let Ok(status) = self.status.try_borrow() {
            match *status {
                Status::Connected | Status::Ok => self.parent_stack.set_visible_child(&self.alt_wid),
                _ => self.parent_stack.set_visible_child_name("status"),
            }
        } else {
            println!("Could not borrow status");
        }
    }

    /// Updates the status. If the status is of type Ok, show the alternative
    /// widget. If status is not ok, show the child widget with the corresponding
    /// error.
    pub fn update(&self, status : Status) {
        if let Ok(mut old_status) = self.status.try_borrow_mut() {
            *old_status = status.clone();
        } else {
            println!("Failed to retrieve mutable reference to status");
            return;
        }
        match status {
            Status::Ok => {
                self.parent_stack.set_visible_child(&self.alt_wid);
                return;
            },
            status => {
                self.parent_stack.set_visible_child(&self.status_stack);
                self.status_stack.set_visible_child(&self.status_boxes[status.index()]);
                match status {
                    Status::StatementExecuted(txt) => {
                        self.stmt_label.set_text(&txt[..])
                    },
                    Status::SqlErr(txt) => {
                        self.sql_err_label.set_text(&txt[..])
                    },
                    Status::ConnectionErr(txt) => {
                        self.conn_err_label.set_text(&txt[..])
                    },
                    _ => { }
                }
            }
        }
    }

}

