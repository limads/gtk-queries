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
use crate::utils;
use sourceview::View;
use super::sql_editor::SqlEditor;
use std::path::{Path, PathBuf};
use glib::{types::Type, value::{Value, ToValue}};
use gdk_pixbuf::Pixbuf;
use std::collections::HashMap;
//use either::Either;

/*/// Implemented by types which can be viewed by modifying the given widget
/// (assumed to be a wrapped pointer)
pub trait Show<W>
where
    W : WidgetExt
{
    fn show(&self, wid : &W);
}

impl Show<Image> for DBObject {

    fn show(&self, wid : &Image) {
        match self {
            DBObject::Schema{ name, children } => {

            },
            DBObject::Table{ name, cols } => {

            }
        }
    }

}

impl Show<Label> for DBObject {

    fn show(&self, wid : &Label) {
        match self {
            DBObject::Schema{ name, children } => {

            },
            DBObject::Table{ name, cols } => {

            }
        }
    }

}*/

pub enum Growth<T> {
    Depth(T),
    Breadth(T),
    Halt
}

// IconTree<T : Display + Iterator<Item=Growth<&Self, &Self>>>
// icontree::build(.) then takes a HashMap<String, Pixbuf> at its initialization.
// matching left grows the tree in a depth fashion; matching right rows the
// tree in a breadth fashion.
#[derive(Clone)]
pub struct SchemaTree {
    tree_view : TreeView,
    model : TreeStore,
    type_icons : HashMap<DBType, Pixbuf>,
    tbl_icon : Pixbuf,
    schema_icon : Pixbuf
}

const ALL_TYPES : [DBType; 15] = [
    DBType::Bool,
    DBType::I16,
    DBType::I32,
    DBType::I64,
    DBType::F32,
    DBType::F64,
    DBType::Numeric,
    DBType::Text,
    DBType::Date,
    DBType::Time,
    DBType::Bytes,
    DBType::Json,
    DBType::Xml,
    DBType::Array,
    DBType::Unknown
];

impl SchemaTree {

    fn load_type_icons() -> HashMap<DBType, Pixbuf> {
        let mut type_icons = HashMap::new();
        for ty in ALL_TYPES.iter() {
            let path = match ty {
                DBType::Bool => "boolean.svg",
                DBType::I16 | DBType::I32 | DBType::I64 => "integer.svg",
                DBType::F32 | DBType::F64 | DBType::Numeric => "real.svg",
                DBType::Text => "text.svg",
                DBType::Date => "date.svg",
                DBType::Time => "time.svg",
                DBType::Json => "json.svg",
                DBType::Xml => "xml.svg",
                DBType::Bytes => "binary.svg",
                DBType::Array => "array.svg",
                DBType::Unknown => "unknown.svg"
            };
            let pix = Pixbuf::from_file_at_scale(&format!("assets/icons/types/{}", path), 16, 16, true).unwrap();
            type_icons.insert(*ty, pix);
        }
        type_icons
    }

    pub fn build(builder : &Builder) -> Self {
        let type_icons = Self::load_type_icons();
        let tbl_icon = Pixbuf::from_file_at_scale("assets/icons/grid-black.svg", 16, 16, true).unwrap();
        let schema_icon = Pixbuf::from_file_at_scale("assets/icons/db.svg", 16, 16, true).unwrap();
        let tree_view : TreeView = builder.get_object("schema_tree_view").unwrap();
        let model = utils::configure_tree_view(&tree_view);
        Self{ tree_view, model, type_icons, tbl_icon, schema_icon }
    }

    // grow_tree<T>(obj : T) for T : Display + Iterator<Item=&Self>
    // and receive a HashMap<&str, Pixbuf> which maps the Display key to a Pixbuf living at this hash.
    fn grow_tree(&self, model : &TreeStore, parent : Option<&TreeIter>, obj : DBObject) {
        match obj {
            DBObject::Schema{ name, children } => {
                println!("Adding schema {:?} to model", name);
                let schema_pos = model.append(parent);
                model.set(&schema_pos, &[0, 1], &[&self.schema_icon, &name.to_value()]);
                for child in children {
                    self.grow_tree(&model, Some(&schema_pos), child);
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
        let mut is_pg = false;
        if let Ok(t_env) = tbl_env.try_borrow() {
            if let Some(objs) = t_env.db_info() {
                if &t_env.get_engine_name()[..] == "PostgreSQL" {
                    is_pg = true;
                }
                for obj in objs {
                    self.grow_tree(&self.model, None, /*self.model.get_iter_first().as_ref()*/ obj);
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
        if is_pg {
            self.model.foreach(|model, path, iter| {
                if path.get_depth() == 1 {
                    self.tree_view.expand_row(path, false);
                }
                false
            });
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


