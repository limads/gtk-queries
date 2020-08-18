use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell};
use std::fs::File;
use std::io::Read;
use gdk::{self, keys};
use crate::tables::environment::{TableEnvironment, EnvironmentUpdate, DBObject, DBType};
use sourceview::*;
use gtk::prelude::*;
use crate::{status_stack::StatusStack};
use crate::status_stack::*;
use sourceview::View;
use super::sql_editor::SqlEditor;
use std::path::{Path, PathBuf};
use glib::{types::Type, value::{Value, ToValue}};

#[derive(Clone)]
pub struct SchemaTree {
    tree_view : TreeView
}

impl SchemaTree {

    pub fn build(builder : &Builder) -> Self {
        let tree_view : TreeView = builder.get_object("schema_tree_view").unwrap();
        Self{ tree_view }
    }

    fn grow_schema(model : &TreeStore, parent : Option<&TreeIter>, obj : DBObject) {
        match obj {
            DBObject::Schema{ name, children } => {
                let schema_pos = model.append(parent);
                model.set(&schema_pos, &[0], &[&name.to_value()]);
                for child in children {
                    Self::grow_schema(&model, Some(&schema_pos), child);
                }
            },
            DBObject::Table{ name, cols } => {
                let tbl_pos = model.append(parent);
                model.set(&tbl_pos, &[0], &[&name.to_value()]);
                for c in cols {
                    let col_pos = model.append(Some(&tbl_pos));
                    model.set(&col_pos, &[0], &[&c.0.to_value()]);
                }
            }
        }
    }

    pub fn repopulate(&self, tbl_env : Rc<RefCell<TableEnvironment>>) {
        self.clear();
        if let Ok(t_env) = tbl_env.try_borrow() {
            if let Some(objs) = t_env.db_info() {
                let model = TreeStore::new(&[Type::String]);
                for obj in objs {
                    Self::grow_schema(&model, None, obj);
                }
                self.tree_view.set_model(Some(&model));
                self.tree_view.show_all();
            } else {
                println!("Unable to acquire database objects");
            }
        } else {
            println!("Failed acquiring reference to table environment");
        }
        self.tree_view.show_all();
    }

    pub fn clear(&self) {
        // for child in self.tree_view.get_children() {
        //    self.tree_view.remove_child(&child);
        // }
        self.tree_view.set_model(None::<&TreeStore>);
        self.tree_view.show_all();
    }

}


