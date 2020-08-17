use gtk::*;
use gio::prelude::*;
use std::env::{ /*self,*/ args};
use std::rc::Rc;
use std::cell::{RefCell /*, RefMut*/ };
use std::fs::File;
use std::io::Write;
use gdk::{self, keys};
use crate::tables::{source::EnvironmentSource, environment::TableEnvironment, environment::EnvironmentUpdate, /*sql::SqlListener*/ };
use crate::conn_popover::*;
use sourceview::*;
use gtk::prelude::*;
use crate::{utils, /*table_widget::TableWidget,*/ table_notebook::TableNotebook };
use crate::status_stack::*;
use crate::sql_editor::*;

pub struct QuerySidebar {
    sql_list : ListBox,
    schema_list : ListBox
}

impl QuerySidebar {

    pub fn new(builder : &Builder) -> Self {
        let sql_list : ListBox = builder.get_object("sql_list_box").unwrap();
        let schema_list : ListBox = builder.get_object("schema_list_box").unwrap();
        Self{ sql_list, schema_list }
    }

}


