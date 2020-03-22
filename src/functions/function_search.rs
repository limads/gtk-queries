use gtk::*;
use gio::prelude::*;
use std::env::{self, args};
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::collections::HashMap;
use std::path::PathBuf;
use std::ffi::OsStr;
use gdk::ModifierType;
use gdk::{self, enums::key};
use tables::{self, environment_source::EnvironmentSource, TableEnvironment, button::TableChooser, sql::SqlListener};
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use crate::{utils, table_widget::TableWidget, table_notebook::TableNotebook };
use nlearn::table::Table;
use std::fmt::Display;
use std::io::{Read, Write};
use crate::functions::num_function::*;

/*#[derive(Clone)]
pub struct FunctionViewer<'a> {
    app_xml : &'a str,
    parent : Box,
    fns : Rc<RefCell<Vec<FunctionBox>>>
}*/

#[derive(Clone)]
pub struct FunctionBox {
    /*func : NumFunction*/
}

/*impl<'a> FunctionViewer<'a> {

    pub fn new(app_xml : &'a str, builder : Builder) -> Self {
        let parent : Box = builder.get_object("function_viewer_box").unwrap();
        parent.show_all();
        let fns = Rc::new(RefCell::new(Vec::new()));
        Self { app_xml, parent, fns }
    }

    pub fn new_function(&mut self, name : &str, func : NumFunction) -> Result<(), impl Display> {
        let builder = Builder::new();
        if let Err(e) = builder.add_objects_from_string(
            self.app_xml,
            &["function_box","show_fn_toggle", "fn_name_label", "fn_clear_btn", "fn_erase_btn", "fn_refresh_btn", "image10", "image11", "image12"]
        ) {
            Err(format!("{}", e))
        } else {
            let bx_parent : Box = builder.get_object("function_box").unwrap();
            let name_label : Label = builder.get_object("fn_name_label").unwrap();
            name_label.set_text(&func.name);
            let args_box : Box = builder.get_object("function_viewer_box").unwrap();
            for (arg, val) in func.args.iter() {
                let arg_label = Label::new(Some(&arg));
                let arg_entry = Entry::new();
                let entry_txt = match val {
                    Some(val) => val.to_string(),
                    None => String::from("")
                };
                arg_entry.set_text(&entry_txt[..]);
                let arg_box = Box::new(Orientation::Horizontal, 0);
                arg_box.pack_end(&arg_label, true, true, 0);
                arg_box.pack_end(&arg_entry, true, true, 0);
                args_box.pack_end(&arg_box, true, true, 0);
            }
            args_box.show_all();
            let fn_clear_btn : Button = builder.get_object("fn_clear_btn").unwrap();
            let fn_erase_btn : Button = builder.get_object("fn_erase_btn").unwrap();
            let fn_refresh_btn : Button = builder.get_object("fn_refresh_btn").unwrap();

            {
                fn_clear_btn.connect_clicked(move|_|{ });
            }

            {
                fn_erase_btn.connect_clicked(move|_|{ });
            }

            {
                fn_refresh_btn.connect_clicked(move|_|{ });
            }
            self.parent.pack_end(&bx_parent, true, true, 0);
            bx_parent.show_all();
            self.parent.show_all();
            let fn_box = FunctionBox{ /*func*/ };
            if let Ok(mut fns) = self.fns.try_borrow_mut() {
                fns.push(fn_box);
            } else {
                println!("Could not get mutable reference to functions vector");
            }

            Ok(())
        }
    }
}*/

#[derive(Clone)]
pub struct FunctionSearch {
    search_entry : Entry,
    completion_list_store : ListStore,
    reg : Rc<NumRegistry>,
    update_btn : ToolButton,
    clear_btn : ToolButton,
    curr_fn : Rc<RefCell<Option<(String, HashMap<String, String>)>>>,
    fn_doc_label : Label,
    doc_stack : Stack,
    cols_label : Label
}

impl FunctionSearch {

    /*pub fn connect_with_tbl_list(&self, table_list : TableList) {
        {
            let search_entry = self.search_entry.clone();
            let reg = self.reg.clone();
            let table_list = table_list.clone();
            let src_entry = self.src_entry.clone();
            let dst_entry = self.dst_entry.clone();
            let func_search = self.clone();
            self.fn_update_btn.connect_clicked(move |_| {
                if let Some(name) = search_entry.get_text().map(|n| n.to_string()) {
                    if reg.has_func_name(&name[..]) {
                        let src_txt = src_entry.get_text().map(|t| t.to_string());
                        let dst_txt = dst_entry.get_text().map(|t| t.to_string());
                        match (src_txt, dst_txt) {
                            (Some(src), Some(dst)) => {
                                let tbl_exists = table_list.has_table(&src) && table_list.has_table(&dst);
                                match func_search.read_args() {
                                    Ok(args) => {
                                        let args : Vec<&str> = args.iter().map(|a| &a[..]).collect();
                                        if let Some(func) = reg.retrieve_func(&name[..]) {
                                            let ans = table_list.apply_func(
                                                &src[..],
                                                &dst[..],
                                                &name[..],
                                                &args[..],
                                                func
                                            );
                                            if let Err(e) = ans {
                                                println!("{}", e);
                                            }
                                        } else {
                                            println!("Could not retrieve function {}", name);
                                        }
                                    },
                                    Err(e) => {
                                        println!("{}", e);
                                    }
                                }
                            },
                            _ => {
                                println!("Missing source and/or dst tables");
                            }
                        }
                    }
                }
            });
        }

        {
            let func_search = self.clone();
            let list_box = table_list.list_box.clone();
            let search_entry = self.search_entry.clone();
            let fn_update_btn = self.fn_update_btn.clone();
            let fn_remove_btn = self.fn_remove_btn.clone();
            let fn_calls = table_list.fn_calls.clone();
            let fn_name_label = self.fn_name_label.clone();
            let fn_doc_label = self.fn_doc_label.clone();
            //let fn_arg_box = self.fn_arg_box.clone();
            let reg = self.reg.clone();
            list_box.connect_row_selected(move |ls_bx, opt_row| {
                if let Some(row) = opt_row {
                    if let Ok(fns) = fn_calls.try_borrow() {
                        if let Some(fn_call) = fns.get(row.get_index() as usize) {
                            match fn_call {
                                Some(f) =>  {
                                    search_entry.set_text(&f.name[..]);
                                    fn_name_label.set_text(&f.name[..]);
                                    if let Some(d) = reg.get_doc(&f.name[..]) {
                                        fn_doc_label.set_text(&d[..]);
                                    }
                                    if let Some(params) = reg.get_args(&f.name[..]) {
                                        func_search.update_param_box(&params[..])
                                    }
                                    func_search.update_args(&f.args[..]);
                                },
                                None => {
                                    search_entry.set_text("");
                                    fn_name_label.set_text("");
                                    fn_doc_label.set_text("");
                                    func_search.update_param_box(&[]);
                                }
                            }
                            fn_remove_btn.set_sensitive(true);
                            fn_update_btn.set_sensitive(true);
                        } else {
                            println!("Unable to retrieve function name");
                        }
                    }
                } else {
                    println!("Table list ")
                }
            });
        }
        self.search_entry.connect_focus(move |_entry, _focus_type| {
            table_list.list_box.unselect_all();
            glib::signal::Inhibit(false)
        });
    }*/

    /*fn update_param_box(&self, params : &[String]) {
        for child in self.fn_arg_box.get_children() {
            self.fn_arg_box.remove(&child);
        }
        for param in params {
            let bx = Box::new(Orientation::Horizontal, 0);
            let label = Label::new(Some(param));
            let entry = Entry::new();
            bx.pack_end(&entry, true, true, 0);
            bx.pack_end(&label, true, true, 0);
            self.fn_arg_box.pack_end(&bx, true, true, 0);
        }
        self.fn_arg_box.show_all();
    }*/

    /*fn update_args(&self, args : &[String]) {
        for (entry, arg) in self.get_arg_widgets().iter().zip(args.iter()) {
            entry.set_text(arg);
        }
    }*/

    /*fn get_arg_widgets(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        for (i, child) in self.fn_arg_box.get_children().iter().enumerate() {
            let bx_child : Box = child.clone().downcast().unwrap();
            if let Some(wid) = bx_child.get_children().get(1) {
                let entry : Entry = wid.clone().downcast().unwrap();
                entries.push(entry);
            } else {
                println!("No entry widget");
            }
        }
        entries
    }*/

    /*fn read_args(&self) -> Result<Vec<String>,impl Display> {
        let mut args = Vec::new();
        let entries = self.get_arg_widgets();
        if entries.len() == 0 {
            println!("No arg entries");
        }
        for (i, entry) in entries.iter().enumerate() {
            if let Some(txt) = entry.get_text() {
                args.push(txt.to_string());
            } else {
                return Err(format!("Missing argument at position {}", i));
            }
        }
        Ok(args)
    }*/

    pub fn set_active_status(&self, active : bool) {
        self.search_entry.set_sensitive(active);
        if active {
            self.doc_stack.set_visible_child_name("page2");
        } else {
            self.doc_stack.set_visible_child_name("page0");
        }
    }

    pub fn split_args(call : &str) -> Option<(String, HashMap<String, String>)> {
        let mut txts = call.split_whitespace();
        let name = txts.next()?.to_string();
        let mut args = HashMap::new();
        for arg in txts {
            if !arg.is_empty() && !arg.chars().all(|c| c.is_whitespace() ) {
                let mut arg_pair = arg.split("=");
                let arg_name = arg_pair.next()?;
                let arg_val = arg_pair.next()?;
                if !arg_pair.next().is_none() {
                    return None;
                }
                args.insert(arg_name.to_string(), arg_val.to_string());
            }
        }
        Some((name, args))
    }

    pub fn update_fn_info(&self, call : &str, selected : &[usize]) {
        if selected.len() > 0 {
            self.search_entry.set_sensitive(true);
            if call.is_empty() {
                self.doc_stack.set_visible_child_name("cols_selected");
                self.cols_label.set_text(&format!("{} column(s) selected", selected.len())[..]);
            } else {
                if let Some((name, args)) = Self::split_args(call) {
                    if self.reg.has_func_name(&name[..]) {
                        if let Some(doc) = self.reg.get_doc(&name[..]) {
                            self.doc_stack.set_visible_child_name("fn_doc");
                            self.fn_doc_label.set_text(&doc[..]);
                            if let Ok(mut curr_fn) = self.curr_fn.try_borrow_mut() {
                                *curr_fn = Some((name, args));
                            } else {
                                println!("Unable to retrieve mutable reference to current fn");
                            }
                        } else {
                            self.doc_stack.set_visible_child_name("invalid_call");
                            //println!("No doc available");
                        }
                    } else {
                        println!("{} not in function registry", name);
                        self.doc_stack.set_visible_child_name("invalid_call");
                    }
                } else {
                    self.doc_stack.set_visible_child_name("invalid_call");
                }
            }
        } else {
            self.search_entry.set_sensitive(false);
            self.doc_stack.set_visible_child_name("no_columns");
        }
    }

    pub fn new(builder : Builder, reg : Rc<NumRegistry>, tbl_nb : TableNotebook) -> Self {
        let search_entry : Entry =
            builder.get_object("fn_search").unwrap();
        //let provider = utils::provider_from_path("entry.css").unwrap();
        //let ctx = search_entry.get_style_context();
        //ctx.add_provider(&provider,800);
        let completion : EntryCompletion =
            builder.get_object("fn_completion").unwrap();
        completion.set_text_column(0);
        completion.set_minimum_key_length(1);
        //completion.set_popup_completion(true);
        let completion_list_store : ListStore =
            builder.get_object("fn_list_store").unwrap();
        //let fn_tree_model_filter : TreeModelFilter =
        //    builder.get_object("treemodelfilter1").unwrap();
        // let fn_arg_box : Box = builder.get_object("fn_arg_box").unwrap();
        let fn_doc_label : Label = builder.get_object("fn_doc_label").unwrap();
        let fn_toolbar : Toolbar = builder.get_object("fn_toolbar").unwrap();
        let img_clear = Image::new_from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let img_update = Image::new_from_icon_name(Some("view-refresh"), IconSize::SmallToolbar);
        let clear_btn : ToolButton = ToolButton::new(Some(&img_clear), None);
        let update_btn : ToolButton = ToolButton::new(Some(&img_update), None);
        let cols_label : Label = builder.get_object("cols_label").unwrap();
        let doc_stack : Stack = builder.get_object("fn_doc_stack").unwrap();
        fn_toolbar.insert(&clear_btn, 0);
        fn_toolbar.insert(&update_btn, 1);
        fn_toolbar.show_all();

       {
            let reg = reg.clone();
            search_entry.connect_key_release_event(move |entry, _ev_key| {
                if let Some(name) = entry.get_text().map(|n| n.to_string()) {
                    if reg.has_func_name(&name[..]) {
                        //fn_update_btn.set_sensitive(true);
                    } else {
                        //fn_update_btn.set_sensitive(false);
                    }
                }
                glib::signal::Inhibit(false)
            });
        }

        /*{
            let fn_update_btn : Button = update_btn.clone();
            let fn_remove_btn : Button = fn_remove_btn.clone();
            search_entry.connect_focus(move |_entry, _focus_type| {
                fn_update_btn.set_sensitive(false);
                fn_remove_btn.set_sensitive(false);
                glib::signal::Inhibit(false)
            });
        }*/

        let curr_fn = Rc::new(RefCell::new(None));
        let fn_search = Self {
            search_entry,
            completion_list_store,
            reg,
            update_btn,
            clear_btn,
            fn_doc_label,
            doc_stack,
            curr_fn,
            cols_label
            //fn_arg_box,

        };

        {
            let fn_search = fn_search.clone();
            let search_entry = fn_search.search_entry.clone();
            let tbl_nb = tbl_nb.clone();
            search_entry.connect_key_press_event(move |entry, key_ev| {
                match key_ev.get_keyval() {
                    key::Return => {
                        let text = entry.get_text().map(|txt| txt.to_string()).unwrap_or("".to_string());
                        let selected = tbl_nb.selected_cols();
                        fn_search.update_fn_info(&text[..], &selected[..]);
                        if key_ev.get_state() == gdk::ModifierType::MOD2_MASK {
                            println!("Execute");
                        }
                        glib::signal::Inhibit(true)
                    }
                    _ => {
                        glib::signal::Inhibit(false)
                    }
                }
            });
        }

        {
            let fn_search = fn_search.clone();
            //completion.set_match_func(move |_compl, txt, _iter| {
            //    println!("{}", txt);
            //    true
            //});
            let tbl_nb = tbl_nb.clone();
            completion.connect_match_selected(move |compl, model, iter|{
                if let Ok(Some(text)) = model.get_value(iter, 0).get::<String>() {
                    let selected = tbl_nb.selected_cols();
                    fn_search.update_fn_info(&text[..], &selected[..]);
                }
                //println!("{:?}", ;
                glib::signal::Inhibit(false)
            });
            completion.connect_cursor_on_match(move |compl, _model, _iter|{
                if let Some(prefix) = compl.get_completion_prefix().map(|p| p.to_string()) {
                    println!("{}", prefix);
                }
                glib::signal::Inhibit(false)
            });
        }
        fn_search
    }

    pub fn populate_search(&self, names : Vec<String>) -> Result<(), &'static str> {
        // let name1 = "summary".to_string();
        // let name2 = "fit".to_string();
        // let name3 = "eval".to_string();
        // let mut fn_names = Vec::new();
        // fn_names.push(&name1 as &dyn ToValue);
        // fn_names.push(&name2 as &dyn ToValue);
        // fn_names.push(&name3 as &dyn ToValue);
        let dyn_names : Vec<_> = names.iter().map(|m| m as &dyn ToValue).collect();
        let cols = [0];
        for i in 0..dyn_names.len() {
            self.completion_list_store.insert_with_values(
                Some(i as u32),
                &cols[0..],
                &dyn_names[(i as usize)..((i+1) as usize)]
            );
        }
        Ok(())
    }

}


