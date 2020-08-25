use gtk::*;
use gio::prelude::*;
use std::env::{self, args};
use std::rc::Rc;
use std::cell::{Cell, RefCell, RefMut};
use std::fs::File;
use std::collections::HashMap;
use std::path::PathBuf;
use std::ffi::OsStr;
use gdk::ModifierType;
use gdk::{self, keys};
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use std::fmt::Display;
use std::io::{Read, Write};
use super::sql_type::*;
use super::loader::*;
use super::function::*;
use std::sync::{Arc, Mutex};

// use crate::table_list::*;
/*#[derive(Clone)]
pub struct FunctionViewer<'a> {
    app_xml : &'a str,
    parent : Box,
    fns : Rc<RefCell<Vec<FunctionBox>>>
}*/

//#[derive(Clone)]
//pub struct FunctionBox {
/*func : NumFunction*/
//}

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
pub struct FunctionRegistry {
    search_entry : Entry,
    // completion_list_store : ListStore,
    lib_list_box : ListBox,
    fn_list_box : ListBox,
    loader : Arc<Mutex<FunctionLoader>>,
    lib_update_btn : Button,
    lib_remove_btn : Button,
    lib_add_btn : Button,
    fn_name_label : Label,
    fn_doc_label : Label,
    so_file_chooser : FileChooserDialog,
    lib_info : InfoBar,
    info_lbl : Label,
    sensitive : Rc<Cell<bool>>
    // fn_arg_box : Box,
    // src_entry : Entry,
    // dst_entry : Entry
}

impl FunctionRegistry {

    /*pub fn connect_with_tbl_list(&self, table_list : TableList) {
        {
            let search_entry = self.search_entry.clone();
            let loader = self.loader.clone();
            let table_list = table_list.clone();
            let src_entry = self.src_entry.clone();
            let dst_entry = self.dst_entry.clone();
            let func_search = self.clone();
            self.lib_update_btn.connect_clicked(move |_| {
                if let Some(name) = search_entry.get_text().map(|n| n.to_string()) {
                    if loader.has_func_name(&name[..]) {
                        let src_txt = src_entry.get_text().map(|t| t.to_string());
                        let dst_txt = dst_entry.get_text().map(|t| t.to_string());
                        match (src_txt, dst_txt) {
                            (Some(src), Some(dst)) => {
                                let tbl_exists = table_list.has_table(&src) && table_list.has_table(&dst);
                                match func_search.read_args() {
                                    Ok(args) => {
                                        let args : Vec<&str> = args.iter().map(|a| &a[..]).collect();
                                        if let Some(func) = loader.retrieve_func(&name[..]) {
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
            let lib_update_btn = self.lib_update_btn.clone();
            let lib_remove_btn = self.lib_remove_btn.clone();
            let fn_calls = table_list.fn_calls.clone();
            let fn_name_label = self.fn_name_label.clone();
            let fn_doc_label = self.fn_doc_label.clone();
            //let fn_arg_box = self.fn_arg_box.clone();
            let loader = self.loader.clone();
            list_box.connect_row_selected(move |ls_bx, opt_row| {
                if let Some(row) = opt_row {
                    if let Ok(fns) = fn_calls.try_borrow() {
                        if let Some(fn_call) = fns.get(row.get_index() as usize) {
                            match fn_call {
                                Some(f) =>  {
                                    search_entry.set_text(&f.name[..]);
                                    fn_name_label.set_text(&f.name[..]);
                                    if let Some(d) = loader.get_doc(&f.name[..]) {
                                        fn_doc_label.set_text(&d[..]);
                                    }
                                    if let Some(params) = loader.get_args(&f.name[..]) {
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
                            lib_remove_btn.set_sensitive(true);
                            lib_update_btn.set_sensitive(true);
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
    }

    fn get_arg_widgets(&self) -> Vec<Entry> {
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

    pub fn set_sensitive(&self, state : bool) {
        self.sensitive.set(state);
        self.lib_update_btn.set_sensitive(state);
        self.lib_add_btn.set_sensitive(state);
        self.lib_remove_btn.set_sensitive(state);
        for child in self.lib_list_box.get_children() {
            let row = child.downcast::<ListBoxRow>().unwrap();
            let wid = row.get_child().unwrap();
            let bx = wid.downcast::<gtk::Box>().unwrap();
            let check = &bx.get_children()[0].clone()
                .downcast::<CheckButton>().unwrap();
            check.set_sensitive(state);
        }
    }

    fn update_fn_info(&self, name : Option<&str>) {
        if let Ok(loader) = self.loader.lock() {
            if let Some(name) = name {
                if loader.has_func_name(&name[..]) {
                    self.fn_name_label.set_text(name);
                    if let Some(doc) = loader.get_doc(name) {
                        self.fn_doc_label.set_text(&doc[..]);
                    } else {
                        self.fn_doc_label.set_text("No documentation available");
                    }
                } else {
                    println!("{} not in function Registry", name);
                }
            } else {
                self.fn_name_label.set_text("");
                self.fn_doc_label.set_text("");
            }
        } else {
            println!("Unable to borrow loader");
        }
    }

    fn reload_lib_list(
        lib_list_box : &ListBox,
        fn_list_box : &ListBox,
        loader : &Arc<Mutex<FunctionLoader>>,
        prefix : Option<&str>
    ) {
        for child in lib_list_box.get_children() {
            lib_list_box.remove(&child);
        }
        for child in fn_list_box.get_children() {
            fn_list_box.remove(&child);
        }
        let lib_list : Vec<(String, bool)> = if let Ok(loader) = loader.lock() {
            loader.lib_list()
                .iter()
                .map(|lib| (lib.name.to_string(), lib.active) )
                .collect()
        } else {
            println!("Failed acquiring lock over function loader");
            Vec::new()
        };
        for (i, (lib, active)) in lib_list.iter().enumerate() {
            if prefix.map(|p| lib.starts_with(p) ).unwrap_or(true) {
                // println!("Must add: {}", lib);
                let n = lib_list_box.get_children().len();
                lib_list_box.insert(&Self::build_lib_item(lib, &loader, *active), n as i32);
            }
        }
        lib_list_box.show_all();
    }

    fn build_lib_item(name : &str, loader : &Arc<Mutex<FunctionLoader>>, active : bool) -> ListBoxRow {
        let bx = Box::new(Orientation::Horizontal, 0);
        let check = CheckButton::new();
        check.set_active(active);
        let loader = loader.clone();
        let name_string = name.to_string();
        check.connect_toggled(move |btn| {
            if let Ok(mut loader) = loader.lock() {
                if let Err(e) = loader.set_active_status(&name_string[..], btn.get_active()) {
                    println!("{}", e);
                }
            } else {
                println!("Not possible to lock loader");
            }
        });
        let lbl = Label::new(Some(name));
        bx.pack_start(&check, false, false, 0);
        bx.pack_start(&lbl, false, false, 0);
        let row = ListBoxRow::new();
        row.add(&bx);
        row.set_selectable(true);
        row
    }

    fn get_row_name(row : &ListBoxRow, label_ix : usize) -> Option<String> {
        let child = row.get_child()?;
        let child_box = child.downcast::<Box>().unwrap();
        let label = child_box.get_children().get(label_ix)?.clone()
            .downcast::<Label>().ok()?;
        Some(label.get_text().as_str().to_string())
    }

    fn reload_fn_list(list_box : &ListBox, funcs : &[&Function]) {
        for child in list_box.get_children() {
            list_box.remove(&child);
        }
        for (i, f) in funcs.iter().enumerate() {
            println!("Must add function: {:?}", f);
            let n = list_box.get_children().len();
            list_box.insert(&Label::new(Some(&f.name[..])), n as i32);
        }
        list_box.show_all();
    }

    //pub fn append_new_item(&self, tbl_name : &str, sz : (usize, usize)) {
    //    let n = self.list_box.get_children().len();
    //    self.list_box.insert(&Self::build_list_item(tbl_name, sz), n as i32);
    //}

    /*fn build_list_item(name : &str, sz : (usize, usize)) -> Box {
        let img_name = match sz {
            (1, _) => "table_row",
            (_, 1) => "table_col",
            _ => "table_full"
        };
        let tab_img_path = String::from("assets/icons/") + img_name + ".svg";
        let img = Image::new_from_file(&tab_img_path[..]);
        let label = Label::new(Some(name));
        let bx = Box::new(Orientation::Horizontal, 0);
        bx.pack_start(&label, true, true, 0);
        bx.pack_start(&img, true, true, 0);
        bx
    }*/

    pub fn new(builder : &Builder) -> (Self, Arc<Mutex<FunctionLoader>>) {
        let search_entry : Entry =
            builder.get_object("function_search_entry").unwrap();
        let loader = Arc::new(Mutex::new(FunctionLoader::load().map_err(|e| { println!("{}", e); e }).unwrap()));
        let lib_info : InfoBar = builder.get_object("lib_info").unwrap();
        let info_lbl : Label = builder.get_object("info_label").unwrap();
        let lib_list_box : ListBox = builder.get_object("lib_list_box").unwrap();
        let fn_list_box : ListBox = builder.get_object("fn_list_box").unwrap();
        let lib_add_btn : Button = builder.get_object("lib_add_btn").unwrap();
        let lib_remove_btn : Button = builder.get_object("lib_remove_btn").unwrap();
        let lib_update_btn : Button = builder.get_object("lib_update_btn").unwrap();
        let fn_name_label : Label = builder.get_object("fn_name_label").unwrap();
        let fn_doc_label : Label = builder.get_object("fn_doc_label").unwrap();
        let so_file_chooser : FileChooserDialog = builder.get_object("so_file_chooser").unwrap();
        let sensitive = Rc::new(Cell::new(false));
        {
            let loader = loader.clone();
            let lib_list_box = lib_list_box.clone();
            let search_entry = search_entry.clone();
            let lib_info = lib_info.clone();
            let info_lbl = info_lbl.clone();
            let fn_list_box = fn_list_box.clone();
            so_file_chooser.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename().as_ref().and_then(|p| p.to_str() ) {
                            if let Ok(mut loader) = loader.lock() {
                                if let Err(e) = loader.add_crate(&path[..]) {
                                    println!("{}", e);
                                    lib_info.set_visible(true);
                                    lib_info.set_message_type(MessageType::Error);
                                    info_lbl.set_text(&format!("{}", e));
                                } else {
                                    lib_info.set_message_type(MessageType::Info);
                                    lib_info.set_visible(true);
                                    info_lbl.set_text("Library loaded");
                                    println!("Library loaded");
                                }
                            } else {
                                println!("Could not lock function loader");
                            }
                            search_entry.set_text("");
                            Self::reload_lib_list(&lib_list_box, &fn_list_box, &loader, None);
                        } else {
                            println!("Could not retrieve file path");
                        }
                    },
                    _ => { }
                }
            });
        }

        {
            let so_file_chooser = so_file_chooser.clone();
            lib_add_btn.connect_clicked(move |_| {
                so_file_chooser.run();
                so_file_chooser.hide();
            });
        }

        {
            let loader = loader.clone();
            let lib_list_box = lib_list_box.clone();
            let fn_list_box = fn_list_box.clone();
            lib_update_btn.connect_clicked(move |_| {
                if let Ok(mut loader) = loader.lock() {
                    if let Err(e) = loader.reload_libs() {
                        println!("Error updating library: {}", e);
                        return;
                    }
                    println!("Libraries updated");
                } else {
                    println!("Could not lock function loader");
                }
                Self::reload_lib_list(&lib_list_box, &fn_list_box, &loader, None);
            });
        }

        {
            let loader = loader.clone();
            let lib_list_box = lib_list_box.clone();
            let fn_list_box = fn_list_box.clone();
            lib_remove_btn.connect_clicked(move |_| {
                match (loader.lock(), lib_list_box.get_selected_row()) {
                    (Ok(mut loader), Some(row)) => {
                        if let Some(name) = Self::get_row_name(&row, 1) {
                            if let Err(e) = loader.remove_crate(&name[..]) {
                                println!("Error removing library: {}", e);
                                return;
                            }
                            if let Err(e) = loader.reload_libs() {
                                println!("{}", e);
                            }
                        }
                    },
                    _ => {
                        println!("Error removing function");
                    }
                }
                Self::reload_lib_list(&lib_list_box, &fn_list_box, &loader, None);
            });
        }

        {
            let lib_update_btn : Button = lib_update_btn.clone();
            let lib_remove_btn : Button = lib_remove_btn.clone();
            search_entry.connect_focus(move |_entry, _focus_type| {
                // lib_update_btn.set_sensitive(false);
                // lib_remove_btn.set_sensitive(false);
                glib::signal::Inhibit(false)
            });
        }

        {
            let fn_list_box = fn_list_box.clone();
            let loader = loader.clone();
            let (lib_remove_btn, lib_update_btn) = (lib_remove_btn.clone(), lib_update_btn.clone());
            let sensitive = sensitive.clone();
            lib_list_box.connect_row_selected(move|ls_bx, opt_row| {
                if sensitive.get() {
                    lib_remove_btn.set_sensitive(true);
                    lib_update_btn.set_sensitive(true);
                }
                if let Ok(loader) = loader.lock() {
                    if let Some(row) = opt_row {
                        if let Some(child) = row.get_child() {
                            let rbox = child.downcast::<Box>().unwrap();
                            let label = rbox.get_children().get(1).map(|w| w.clone() ).unwrap().downcast::<Label>().unwrap();
                            //if let Ok(child) =  {
                            let text = label.get_text().to_string();
                            let funcs = loader.fn_list_for_lib(&text[..]);
                            println!("Function list for {}: {:?}", text, funcs);
                            Self::reload_fn_list(&fn_list_box, &funcs[..]);

                            //else {
                            //    Self::reload_fn_list(&fn_list_box, &[]);
                            //}
                            //} else {
                            //    println!("No child at row");
                            //}
                        } else {
                            println!("Selected row does not have header");
                        }
                    } else {
                        println!("No row selected");
                    }
                } else {
                    println!("Could not lock function loader");
                }
            });
        }

        {
            let fn_list_box = fn_list_box.clone();
            let (lib_remove_btn, lib_update_btn) = (lib_remove_btn.clone(), lib_update_btn.clone());
            lib_list_box.connect_unselect_all(move |_| {
                lib_remove_btn.set_sensitive(false);
                lib_update_btn.set_sensitive(false);
                Self::reload_fn_list(&fn_list_box, &[]);
            });
        }

        let fn_reg = Self {
            search_entry : search_entry.clone(),
            loader : loader.clone(),
            lib_list_box : lib_list_box.clone(),
            fn_list_box : fn_list_box.clone(),
            lib_add_btn,
            lib_update_btn : lib_update_btn.clone(),
            lib_remove_btn,
            fn_name_label,
            fn_doc_label,
            so_file_chooser,
            info_lbl,
            lib_info,
            sensitive
        };
        Self::reload_lib_list(&fn_reg.lib_list_box, &fn_reg.fn_list_box, &fn_reg.loader, None);

        {
            let fn_reg = fn_reg.clone();
            let lib_list_box = fn_reg.lib_list_box.clone();
            fn_list_box.connect_row_selected(move |ls_bx, opt_row| {
                if let Some(row) = opt_row {
                    if let Some(label) = row.get_child().and_then(|child| child.downcast::<gtk::Label>().ok() ) {
                        //let label = child.get_children()[1].clone().downcast::<Label>().unwrap();
                        let text = label.get_text().to_string();
                        if text.len() >= 1 {
                            fn_reg.update_fn_info(Some(&text[..]));
                        } else {
                            fn_reg.update_fn_info(None);
                        }
                    } else {
                        println!("Row does not have label child");
                    }
                } else {
                    fn_reg.update_fn_info(None);
                }
            });
        }

        {
            let lib_update_btn = lib_update_btn.clone();
            let loader = fn_reg.loader.clone();
            let lib_list_box = fn_reg.lib_list_box.clone();
            let fn_list_box = fn_list_box.clone();
            search_entry.connect_key_release_event(move |entry, _ev_key| {
                let name = entry.get_text().to_string();
                if name.len() >= 1 {
                    Self::reload_lib_list(&lib_list_box, &fn_list_box, &loader, Some(&name[..]));
                } else {
                    Self::reload_lib_list(&lib_list_box, &fn_list_box, &loader, None);
                }
                glib::signal::Inhibit(false)
            });
        }
        (fn_reg, loader)
    }

    /*pub fn populate_search(&self, names : Vec<String>) -> Result<(), &'static str> {
        //let name1 = "summary".to_string();
        //let name2 = "fit".to_string();
        //let name3 = "eval".to_string();
        //let mut fn_names = Vec::new();
        //fn_names.push(&name1 as &dyn ToValue);
        //fn_names.push(&name2 as &dyn ToValue);
        //fn_names.push(&name3 as &dyn ToValue);
        let dyn_names : Vec<_> = names.iter().map(|m| m as &dyn ToValue).collect();
        let cols = [0];
        for i in 0..dyn_names.len() {
            //self.completion_list_store.insert_with_values(
            //    Some(i as u32),
            //    &cols[0..],
            //    &dyn_names[(i as usize)..((i+1) as usize)]
            //);
        }
        Ok(())
    }*/

}


