use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell};
use std::fs::File;
use std::io::Read;
use gdk::{self, enums::key};
use crate::tables::{environment::TableEnvironment, environment::EnvironmentUpdate};
use sourceview::*;
use gtk::prelude::*;
use crate::{status_stack::StatusStack};
use crate::status_stack::*;
use sourceview::View;
use super::sql_editor::SqlEditor;

#[derive(Clone)]
pub struct FileList {
    // Holds the ordered file paths
    files : Rc<RefCell<Vec<String>>>,
    last_ix : Rc<RefCell<usize>>,
    list_box : ListBox
}

impl FileList {

    pub fn set_sensitive(&self, state : bool) {
        self.list_box.set_sensitive(state);
    }

    pub fn connect_selected(&self, sql_editor : &SqlEditor, content_stack : Stack, query_toggle : ToggleButton) {
        let sql_editor = sql_editor.clone();
        let files = self.files.clone();
        let last_ix = self.last_ix.clone();

        println!("New list element selected");

        self.list_box.connect_row_selected(move |ls_bx, opt_row| {
            if let Some(row) = opt_row {
                // let mut past_ix = last_ix.borrow_mut();
                let curr_ix = row.get_index() as usize;
                println!("Currently selected index: {}", curr_ix);
                if let Ok(mut files) = files.try_borrow_mut() {
                    // *past_ix = curr_ix as usize;

                    // Set visible child only when app is at edit mode. At connect_toggles,
                    // the current index will always be used to set to the right source window.
                    let old_name = content_stack.get_visible_child_name().unwrap();
                    println!("Old name: {}", old_name);
                    let new_name = format!("queries_{}", curr_ix);
                    println!("new name: {}", new_name);
                    // if old_name.starts_with("queries") {
                    if !query_toggle.get_active() {
                        query_toggle.set_active(true);
                    }
                    let new_child = content_stack.get_child_by_name(&new_name).unwrap();
                    content_stack.set_visible_child(&new_child);
                    content_stack.show_all();
                    println!("Set stack visible to {:?}", new_name);
                    let set_name = content_stack.get_visible_child_name().unwrap();
                    println!("New name: {}", set_name);
                    println!("---");
                    // }

                    // Independent of whether at edit mode or not, update the current source
                    // of SqlEditor so the query update presses will read the right text buffer.
                    let new_view = content_stack.get_child_by_name(&new_name)
                        .and_then(|child| child.downcast::<ScrolledWindow>().ok() )
                        .and_then(|sw| {
                            let child = sw.get_child().unwrap();
                            child.downcast::<View>().ok()
                        });
                    if let Some(view) = new_view {
                        sql_editor.update_source(view.clone());
                        println!("Updating SQL editor to {:?}", view);
                    } else {
                        println!("Could not retrieve new view");
                    }
                } else {
                    println!("Failed retrieving reference to file list");
                }
            }
        });
    }

    pub fn build(builder : &Builder) -> Self {
        let files = Rc::new(RefCell::new(vec![String::from("(Untitled)")]));
        let list_box : ListBox = builder.get_object("sql_list_box").unwrap();
        let last_ix = Rc::new(RefCell::new(0));
        let file_list = Self{ files, list_box, last_ix };
        file_list.add_file_row("(Untitled)");
        file_list
    }

    /// Called internally by add_file to fill in a new file row with the file
    /// label and close image.
    fn add_file_row(&self, name : &str) -> ListBoxRow {
        let bx = Box::new(Orientation::Horizontal, 0);
        let lbl = Label::new(Some(name));
        bx.pack_start(&lbl, false, false, 0);
        bx.pack_start(&Box::new(Orientation::Horizontal, 0), true, true, 0);
        let img_close = Image::new_from_icon_name(
            Some("application-exit-symbolic"),
            IconSize::SmallToolbar
        );
        let ev_box = EventBox::new();
        ev_box.add(&img_close);

        {
            ev_box.connect_button_press_event(move |ev_box, ev| {
                println!("Should remove file now");
                glib::signal::Inhibit(true)
            });
        }

        bx.pack_start(&ev_box, false, false, 0);
        let row = ListBoxRow::new();
        row.add(&bx);
        row.set_selectable(true);
        // row.set_margin_top(6);
        // row.set_margin_bottom(6);
        let n = self.list_box.get_children().len();
        self.list_box.insert(&row, n as i32);
        self.list_box.show_all();
        row
    }

    /// Public interface to add a new file to the file list. Called at SqlEditor when
    /// the open Sql file window event yielded a successful SQL file path.
    pub fn add_file(&self, path : &str, content_stack : Stack, query_toggle : ToggleButton, refresh_btn : Button) {
        let mut sql_content = String::new();
        if let Ok(mut f) = File::open(path.clone()) {
            if let Err(e) = f.read_to_string(&mut sql_content) {
                println!("{}", e);
            }
            let row = if let Ok(mut files) = self.files.try_borrow_mut() {
                files.push(path.to_string());
                let n = self.list_box.get_children().len();
                println!("Adding queries_{}", n);
                content_stack.add_named(
                    &SqlEditor::new_source(&sql_content, &refresh_btn),
                    &format!("queries_{}", n)
                );
                self.add_file_row(path)
            } else {
                panic!("Unable to retrieve mutable reference to informed file");
            };
            self.list_box.select_row(Some(&row));
            if !query_toggle.get_active() {
                query_toggle.set_active(true);
            }
        } else {
            println!("Unable to access informed path");
        }
    }

    pub fn get_selected(&self) -> usize {
        let row = self.list_box.get_selected_row()
            .map(|row| row.get_index() as usize )
            .unwrap_or(0);
        println!("Selected row: {}", row);
        row
    }

}


