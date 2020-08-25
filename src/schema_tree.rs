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
use gdk_pixbuf::Pixbuf;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SchemaTree {
    tree_view : TreeView,
    model : TreeStore,
    type_icons : HashMap<DBType, Pixbuf>,
    tbl_icon : Pixbuf,
    schema_icon : Pixbuf
}

const ALL_TYPES : [DBType; 4] = [
    DBType::Integer,
    DBType::Float,
    DBType::Text,
    DBType::Bytes
];

const TYPE_PATHS : [&'static str; 4] = [
    "integer.svg",
    "real.svg",
    "text.svg",
    "binary.svg",
];

impl SchemaTree {

    pub fn build(builder : &Builder) -> Self {
        let mut type_icons = HashMap::new();
        for (ty, path) in ALL_TYPES.iter().zip(TYPE_PATHS.iter()) {
            let pix = Pixbuf::from_file_at_scale(&format!("assets/icons/{}", path), 16, 16, true).unwrap();
            type_icons.insert(*ty, pix);
        }
        let tbl_icon = Pixbuf::from_file_at_scale("assets/icons/grid-black.svg", 16, 16, true).unwrap();
        let schema_icon = Pixbuf::from_file_at_scale("assets/icons/db.svg", 16, 16, true).unwrap();

        let tree_view : TreeView = builder.get_object("schema_tree_view").unwrap();
        let model = TreeStore::new(&[Pixbuf::static_type(), Type::String]);
        tree_view.set_model(Some(&model));
        let pix_renderer = CellRendererPixbuf::new();
        let txt_renderer = CellRendererText::new();
        // renderer.set_property_font(&self, font: Option<&str>)
        // renderer.set_property_foreground_rgba(Some(&gdk::RGBA{ red: 0.0, green : 0.0, blue : 0.0, alpha : 1.0}));
        // let area = CellAreaContext::new();
        // area.add(&renderer);
        // let col = TreeViewColumn::with_area(&area);
        let pix_col = TreeViewColumn::new();
        pix_col.pack_start(&pix_renderer, false);
        pix_col.add_attribute(&pix_renderer, "pixbuf", 0);

        let txt_col = TreeViewColumn::new();
        txt_col.pack_start(&txt_renderer, true);
        txt_col.add_attribute(&txt_renderer, "text", 1);

        tree_view.append_column(&pix_col);
        tree_view.append_column(&txt_col);
        tree_view.set_show_expanders(true);
        Self{ tree_view, model, type_icons, tbl_icon, schema_icon }
    }

    fn grow_schema(&self, model : &TreeStore, parent : Option<&TreeIter>, obj : DBObject) {
        match obj {
            DBObject::Schema{ name, children } => {
                println!("Adding schema {:?} to model", name);
                let schema_pos = model.append(parent);
                model.set(&schema_pos, &[0, 1], &[&self.schema_icon, &name.to_value()]);
                for child in children {
                    self.grow_schema(&model, Some(&schema_pos), child);
                }
            },
            DBObject::Table{ name, cols } => {
                println!("Adding table {:?} to model", name);
                println!("Adding columns {:?} to model", cols);
                let tbl_pos = model.append(parent);
                model.set(&tbl_pos, &[0, 1], &[&self.tbl_icon, &name.to_value()]);
                for c in cols {
                    let col_pos = model.append(Some(&tbl_pos));
                    model.set(&col_pos, &[0, 1], &[&self.type_icons[&c.1], &c.0.to_value()]);
                }
            }
        }
    }

    pub fn repopulate(&self, tbl_env : Rc<RefCell<TableEnvironment>>) {
        self.model.clear();
        if let Ok(t_env) = tbl_env.try_borrow() {
            if let Some(objs) = t_env.db_info() {
                for obj in objs {
                    self.grow_schema(&self.model, None, /*self.model.get_iter_first().as_ref()*/ obj);
                }
                println!("Final model: {:?}", self.model);
                self.model.foreach(|model, path, iter| {
                    println!("Model has path: {:?}", model.get_value(iter, 0).get::<String>()); false
                });
                // self.tree_view.set_model(Some(&model));
                // self.tree_view.expand_all();
                self.tree_view.show_all();
                println!("Using model: {:?}", self.tree_view.get_model());
            } else {
                println!("Unable to acquire database objects");
            }
        } else {
            println!("Failed acquiring reference to table environment");
        }
    }

    pub fn clear(&self) {
        // for child in self.tree_view.get_children() {
        //    self.tree_view.remove_child(&child);
        // }
        // self.tree_view.set_model(None::<&TreeStore>);
        self.model.clear();
        self.tree_view.show_all();
    }

}


