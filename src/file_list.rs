use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell};
use std::fs::File;
use std::io::Read;
use gdk::{self, keys};
use crate::tables::{environment::TableEnvironment, environment::EnvironmentUpdate};
use sourceview::*;
use gtk::prelude::*;
use crate::{status_stack::StatusStack};
use crate::status_stack::*;
use sourceview::View;
use super::sql_editor::SqlEditor;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SqlFile {

    // Name that appear on the sidebar list.
    // Usually the file and its parent directory.
    name : String,

    // Empty if user pressed "New" instead of loading the file from disk.
    path : Option<PathBuf>,

    // Whether the user saved the queries to disk since the last edits.
    saved : bool
}

#[derive(Clone)]
pub struct FileList {
    // Holds the ordered file paths
    files : Rc<RefCell<Vec<SqlFile>>>,
    last_ix : Rc<RefCell<usize>>,
    list_box : ListBox,
    close_confirm_dialog : Dialog,
}

impl FileList {

    pub fn set_sensitive(&self, state : bool) {
        self.list_box.set_sensitive(state);
    }

    pub fn current_selected_path(&self) -> Option<PathBuf> {
        if let Some(sel_ix) = self.get_selected() {
            if let Ok(files) = self.files.try_borrow() {
                files.get(sel_ix).and_then(|f| f.path.clone() )
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_label_from_row(row : &ListBoxRow) -> Label {
        let bx_child = row.get_child().unwrap().downcast::<Box>().unwrap();
        let lbl_child = bx_child
            .get_children()[0]
            .clone()
            .downcast::<Label>().unwrap();
        lbl_child
    }

     pub fn mark_current_saved(&self) {
        if let Some(row) = self.list_box.get_selected_row() {
            let lbl = Self::get_label_from_row(&row);
            let txt = lbl.get_text();
            if txt.as_str().ends_with("*") {
                lbl.set_text(&txt[0..(txt.len()-1)]);
            }
        } else {
            println!("No selected row");
        }
    }

    pub fn mark_current_unsaved(&self) {
        if let Some(row) = self.list_box.get_selected_row() {
            let lbl = Self::get_label_from_row(&row);
            let txt = lbl.get_text();
            if !txt.as_str().ends_with("*") {
                lbl.set_text(&format!("{}*", txt));
            } else {
                // println!("Text already marked as unsaved");
            }
        } else {
            println!("No selected row");
        }
    }

    pub fn set_current_selected_path(&self, path : &Path) {
        if let Some(sel_ix) = self.get_selected() {
            if let Ok(mut files) = self.files.try_borrow_mut() {
                if let Some(mut f) = files.get_mut(sel_ix) {
                    if let Some(name) = SqlEditor::clip_name(&path) {
                        f.name = name.clone();
                        f.path = Some(path.to_path_buf());
                        if let Some(row) = self.list_box.get_selected_row() {
                            let lbl = Self::get_label_from_row(&row);
                            if lbl.get_text().as_str().starts_with("Untitled ") {
                                lbl.set_text(&name);
                            }
                        } else {
                            println!("Unable to get current selected row");
                        }
                    } else {
                        println!("Invalid name");
                    }
                } else {
                    println!("Invalid index: {}", sel_ix);
                }
            } else {
                println!("Could not retrieve mutable reference to file list");
            }
        } else {
            println!("No file selected");
        }
    }

    pub fn connect_selected(
        &self,
        sql_editor : &SqlEditor,
        content_stack : Stack,
        query_toggle : ToggleButton
    ) {
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
                    // let old_name = content_stack.get_visible_child_name().unwrap();
                    // println!("Old name: {}", old_name);
                    let new_name = format!("queries_{}", curr_ix);
                    println!("Selecting: {}", new_name);
                    // if old_name.starts_with("queries") {
                    if !query_toggle.get_active() {
                        query_toggle.set_active(true);
                    }

                    // It is important that the child is visible before setting the visible
                    // child.
                    let new_child = content_stack
                        .get_child_by_name(&new_name)
                        .unwrap();
                    new_child.show();
                    content_stack.set_visible_child(&new_child);
                    content_stack.show_all();
                    content_stack.queue_draw();
                    println!("Set stack visible to {:?}", new_name);
                    let set_name = content_stack.get_visible_child_name().unwrap();
                    println!("Set name: {}", set_name);
                    println!("---");
                    // }

                    // Independent of whether at edit mode or not, update the current source
                    // of SqlEditor so the query update presses will read the right text buffer.
                    sql_editor.update_editor(content_stack.clone(), &new_name);
                } else {
                    println!("Failed retrieving reference to file list");
                }
            }
        });
    }

    pub fn build(builder : &Builder) -> Self {
        let files = Rc::new(RefCell::new(vec![SqlFile{
            name:String::from("Untitled 1"),
            path : None,
            saved : true
        }]));
        let list_box : ListBox = builder.get_object("sql_list_box").unwrap();
        let last_ix = Rc::new(RefCell::new(0));
        let close_confirm_dialog : Dialog = builder.get_object("close_confirm_dialog").unwrap();
        let file_list = Self{ files, list_box, last_ix, close_confirm_dialog };
        file_list
    }

    fn get_n_untitled(&self) -> usize {
        self.files.borrow().iter()
            .filter(|f| f.name.starts_with("Untitled") )
            .filter_map(|f| f.name.split(' ').nth(1) )
            .last()
            .and_then(|n| n.parse::<usize>().ok() )
            .unwrap_or(0)
    }

    pub fn add_fresh_source(
        &self,
        content_stack : Stack,
        sql_editor : SqlEditor,
        query_toggle : ToggleButton
    ) {
        let n_untitled = self.get_n_untitled();
        let title = &format!("Untitled {}", n_untitled + 1);
        println!("New title: {}", title);
        self.files.borrow_mut().push(SqlFile{
            name : title.clone(),
            path : None,
            saved : true
        });
        self.list_box.unselect_all();
        let row = self.add_file_row(&title, content_stack.clone(), sql_editor.clone());
        self.list_box.show_all();
        if !query_toggle.get_active() {
            query_toggle.set_active(true);
        }
        let n = self.files.borrow().len();
        let stack_name = format!("queries_{}", n - 1);
        println!("Adding stack child named: {}", stack_name);
        content_stack.add_named(
            &SqlEditor::new_source("", &sql_editor.refresh_btn.clone(), &self),
            &stack_name
        );
        content_stack.show_all();
        self.list_box.select_row(Some(&row));
    }

    fn remove_source(
        row : ListBoxRow,
        list_box : ListBox,
        files : Rc<RefCell<Vec<SqlFile>>>,
        content_stack : Stack,
        sql_editor : SqlEditor
    ) {
        let ix = row.get_index() as usize;
        let n = list_box.get_children().len();
        list_box.remove(&row);
        if n > 1 {
            let is_selected = content_stack.get_visible_child_name()
                .map(|s| s.as_str().to_string()) == Some(format!("queries_{}", ix));
            files.borrow_mut().remove(ix);
            let curr_child_name = format!("queries_{}", ix);
            println!("Current child name = {}", curr_child_name);
            println!("Current child index = {}", ix);
            let stack_child = content_stack.get_child_by_name(&curr_child_name).unwrap();
            content_stack.remove(&stack_child);
            content_stack.show_all();

            // Case the removed is not last source at stack, rename all posterior sources
            if n >= 2 && ix < n - 1 {
                for i in (ix+1)..n {
                    let old_name = format!("queries_{}", i);
                    let new_name = format!("queries_{}", i - 1);
                    println!("Updating {} to {}", old_name, new_name);
                    let child = content_stack.get_child_by_name(&old_name).unwrap();
                    content_stack.set_child_name(&child, Some(&new_name));
                }
            }

            // It this element was selected before to being removed
            if is_selected && ix >= 1 {
                let prev_name = format!("queries_{}", ix-1);
                println!("Now showing previous name: {}", prev_name);
                content_stack.set_visible_child_name(&prev_name);
                sql_editor.update_editor(content_stack.clone(), &prev_name);
                content_stack.show_all();
            }
        } else {
            // Case this is the last element, just clear the source view and hide it.
            sql_editor.set_text("");
            content_stack.set_visible_child_name("no_queries");
            let lbl_child = Self::get_label_from_row(&row);
            lbl_child.set_text("Untitled 0");
        }
    }

    /// Called internally by add_disk_file and add_fresh_source to fill in a new file row with the file
    /// label and close image. Can also be called externally to add a row
    /// that does not correspond  to a filesystem file (i.e. "Unnamed" entries).
    pub fn add_file_row(
        &self,
        name : &str,
        content_stack : Stack,
        sql_editor : SqlEditor
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        let bx = Box::new(Orientation::Horizontal, 0);
        let lbl = Label::new(Some(name));
        bx.pack_start(&lbl, false, false, 0);
        bx.pack_start(&Box::new(Orientation::Horizontal, 0), true, true, 0);
        let img_close = Image::from_icon_name(
            Some("application-exit-symbolic"),
            IconSize::SmallToolbar
        );
        let ev_box = EventBox::new();
        ev_box.add(&img_close);

        {
            let row = row.clone();
            let list_box = self.list_box.clone();
            let close_confirm_dialog = self.close_confirm_dialog.clone();
            let files = self.files.clone();
            ev_box.connect_button_press_event(move |ev_box, ev| {
                println!("Close button pressed");
                let curr_saved = if let Ok(files) = files.try_borrow() {
                    if let Some(f) = files.get(row.get_index() as usize) {
                        f.saved
                    } else {
                        println!("Unable to get file index");
                        return glib::signal::Inhibit(true);
                    }
                } else {
                    println!("Unable to acquire lock over files");
                    return glib::signal::Inhibit(true);
                };
                if !curr_saved {
                    let ans = close_confirm_dialog.run();
                    if ans != ResponseType::Other(1) {
                        return glib::signal::Inhibit(true);
                    }
                }
                Self::remove_source(
                    row.clone(),
                    list_box.clone(),
                    files.clone(),
                    content_stack.clone(),
                    sql_editor.clone()
                );
                glib::signal::Inhibit(true)
            });
        }

        bx.pack_start(&ev_box, false, false, 0);
        row.add(&bx);
        row.set_selectable(true);
        // row.set_margin_top(6);
        // row.set_margin_bottom(6);
        let n = self.list_box.get_children().len();
        self.list_box.insert(&row, n as i32);
        self.list_box.show_all();
        row.set_property_height_request(24);
        row
    }

    /// Public interface to add a new file to the file list. Called at SqlEditor when
    /// the open Sql file window event yielded a successful SQL file path.
    pub fn add_disk_file(
        &self,
        path : &Path,
        list_name : &str,
        content_stack : Stack,
        query_toggle : ToggleButton,
        refresh_btn : Button,
        sql_editor : SqlEditor
    ) {
        let mut sql_content = String::new();
        if let Ok(mut f) = File::open(&path) {
            if let Err(e) = f.read_to_string(&mut sql_content) {
                println!("{}", e);
            }
            if let Ok(mut files) = self.files.try_borrow_mut() {
                files.push(SqlFile{
                    name : list_name.to_string(),
                    path : Some(path.to_path_buf()),
                    saved : true
                });
            } else {
                println!("Unable to borrow files mutably");
            }
            let n = self.list_box.get_children().len();
            if n == 0 {
                sql_editor.set_text(&sql_content);
                content_stack.set_visible_child_name("queries_0");
            } else {
                let new_name = format!("queries_{}", n);
                println!("Adding {} to content stack", new_name);
                content_stack.add_named(
                    &SqlEditor::new_source(&sql_content, &refresh_btn, &self),
                    &new_name
                );
                content_stack.set_visible_child_name(&new_name);
            }
            let row = self.add_file_row(list_name, content_stack.clone(), sql_editor);
            self.list_box.select_row(Some(&row));
            if !query_toggle.get_active() {
                query_toggle.set_active(true);
            }
        } else {
            println!("Unable to access informed path");
        }
        content_stack.show_all();
        self.select_last();
    }

    pub fn get_selected(&self) -> Option<usize> {
        if self.list_box.get_children().len() == 0 {
            None
        } else {
            let row = self.list_box.get_selected_row()
                .map(|row| row.get_index() as usize )
                .unwrap_or(0);
            println!("Selected row: {:?}", row);
            Some(row)
        }
    }

    pub fn select_last(&self) {
        let n = self.list_box.get_children().len();
        if let Some(row) = self.list_box.get_row_at_index(n as i32 - 1) {
            self.list_box.select_row(Some(&row));
        } else {
            println!("No row to be selected");
        }
    }

}


