use gdk;
use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;
use crate::utils::RecentList;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::process::Command;
use std::path::Path;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::tables::environment::TableEnvironment;
use crate::table_notebook::TableNotebook;
use glib;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use crate::tables::table::{Format, TableSettings, NullField, BoolField, Align};
use std::default::Default;
use crate::utils;
use crate::status_stack::StatusStack;
use std::io::BufWriter;
use std::io::Read;
use crate::command::{self, *};

#[derive(Clone)]
pub struct CsvWindow {
    tbl_bool_combo : ComboBoxText,
    tbl_null_combo : ComboBoxText,
    tbl_prec_spin : SpinButton,
    align_left_radio : RadioButton,
    align_center_radio : RadioButton,
    align_right_radio : RadioButton,
    settings : Rc<RefCell<TableSettings>>
}

impl CsvWindow {

    pub fn build(builder : &Builder) -> Self {
        let settings : Rc<RefCell<TableSettings>> = Rc::new(RefCell::new(Default::default()));
        let tbl_bool_combo : ComboBoxText = builder.get_object("tbl_bool_combo").unwrap();
        {
            let settings = settings.clone();
            tbl_bool_combo.connect_changed(move |combo| {
                if let Some(txt) = combo.get_active_text().map(|s| s.to_string() ) {
                    if let Ok(bool_field) = BoolField::from_str(&txt[..]) {
                        settings.borrow_mut().bool_field = bool_field;
                    } else {
                        println!("Unable to parse format");
                    }
                }
            });
        }

        let tbl_null_combo : ComboBoxText = builder.get_object("tbl_null_combo").unwrap();
        {
            let settings = settings.clone();
            tbl_null_combo.connect_changed(move |combo| {
                if let Some(txt) = combo.get_active_text().map(|s| s.to_string() ) {
                    if let Ok(null) = NullField::from_str(&txt[..]) {
                        settings.borrow_mut().null_field = null;
                    } else {
                        println!("Unable to parse format");
                    }
                }
            });
        }

        let tbl_prec_spin : SpinButton = builder.get_object("tbl_prec_spin").unwrap();
        {
            let settings = settings.clone();
            tbl_prec_spin.connect_preedit_changed(move |_spin, txt| {
                if let Ok(prec) = txt.parse::<usize>() {
                    settings.borrow_mut().prec = prec;
                } else {
                    println!("Unable to parse text as usize");
                }
            });
        }

        let align_left_radio : RadioButton = builder.get_object("align_left_radio").unwrap();
        {
            let settings = settings.clone();
            align_left_radio.connect_group_changed(move |_radio| {
                settings.borrow_mut().align = Align::Left;
            });
        }
        let align_center_radio : RadioButton = builder.get_object("align_left_radio").unwrap();
        {
            let settings = settings.clone();
            align_left_radio.connect_group_changed(move |_radio| {
                settings.borrow_mut().align = Align::Center;
            });
        }
        let align_right_radio : RadioButton = builder.get_object("align_left_radio").unwrap();
        {
            let settings = settings.clone();
            align_left_radio.connect_group_changed(move |_radio| {
                settings.borrow_mut().align = Align::Right;
            });
        }
        Self {
            tbl_bool_combo,
            tbl_null_combo,
            tbl_prec_spin,
            align_left_radio,
            align_center_radio,
            align_right_radio,
            settings
        }
    }
}

#[derive(Clone, Copy)]
enum Destination {
    File,
    Program,
    Clipboard
}

#[derive(Clone)]
struct CopyToBox {
    save_tbl_btn : Button,
    script_btn : Button,
    tbl_format_combo : ComboBoxText,
    save_tbl_dialog : FileChooserDialog,
    clipboard_toggle : ToggleButton,
    program_toggle : ToggleButton,
    file_toggle : ToggleButton,
    dst : Rc<RefCell<Destination>>
}

impl CopyToBox {

    fn alternate_destination(
        this : &ToggleButton, 
        others : Vec<ToggleButton>, 
        dst : Rc<RefCell<Destination>>, 
        curr : Destination,
        save_tbl_btn : Button,
        script_btn : Button
    ) {
        this.connect_toggled(move |btn| {
            let mut n_off = 0;
            if btn.get_active() {
                *(dst.borrow_mut()) = curr;
                for btn in others.iter() {
                    if btn.get_active() {
                        btn.set_active(false);
                    } else {
                        n_off += 1;
                    }
                }
            } else {
                n_off += 1;
                others.iter().for_each(|btn| if !btn.get_active() { n_off += 1; } );
            }
            if n_off == 3 {
                save_tbl_btn.set_sensitive(false);
                script_btn.set_sensitive(false);
            } else {
                save_tbl_btn.set_sensitive(true);
                script_btn.set_sensitive(true);
            }
            println!("{}", n_off);
        });
    }

    fn build(
        builder : &Builder,
        tables_nb : &TableNotebook,
        tbl_env : &Rc<RefCell<TableEnvironment>>,
        csv_window : &CsvWindow,
        cmd_window : &CommandWindow
    ) -> Self {

        let tbl_format_combo : ComboBoxText = builder.get_object("tbl_format_combo").unwrap();
        let dst = Rc::new(RefCell::new(Destination::File));
        let clipboard_toggle : ToggleButton = builder.get_object("copy_to_clipboard_toggle").unwrap();
        let program_toggle : ToggleButton = builder.get_object("copy_to_program_toggle").unwrap();
        let file_toggle : ToggleButton = builder.get_object("copy_to_file_toggle").unwrap();
        
        let save_tbl_btn : Button = builder.get_object("save_tbl_btn").unwrap();
        let script_btn : Button = builder.get_object("script_copy_to_btn").unwrap();
        
        file_toggle.set_active(true);
        Self::alternate_destination(
            &file_toggle, 
            vec![clipboard_toggle.clone(), program_toggle.clone()], 
            dst.clone(), 
            Destination::File,
            save_tbl_btn.clone(),
            script_btn.clone()
        );
        Self::alternate_destination(
            &clipboard_toggle, 
            vec![file_toggle.clone(), program_toggle.clone()], 
            dst.clone(), 
            Destination::Clipboard,
            save_tbl_btn.clone(),
            script_btn.clone()
        );
        Self::alternate_destination(
            &program_toggle, 
            vec![clipboard_toggle.clone(), file_toggle.clone()], 
            dst.clone(), 
            Destination::Program,
            save_tbl_btn.clone(),
            script_btn.clone()
        );
        
        {
            let settings = csv_window.settings.clone();
            tbl_format_combo.clone().connect_changed(move |combo| {
                if let Some(txt) = combo.get_active_text().map(|s| s.to_string() ) {
                    if let Ok(fmt) = Format::from_str(&txt[..]) {
                        settings.borrow_mut().format = fmt;
                    } else {
                        println!("Unable to parse format");
                    }
                }
            });
        }

        let save_tbl_dialog : FileChooserDialog =
            builder.get_object("save_tbl_dialog").unwrap();
        {
            let settings = csv_window.settings.clone();
            let tables_nb = tables_nb.clone();
            let tbl_env = tbl_env.clone();
            save_tbl_dialog.clone().connect_response(move |dialog, resp| {
                let settings = settings.borrow().clone();
                println!("Current table settings: {:?}", settings);
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            let ext = path.as_path()
                                .extension()
                                .map(|ext| ext.to_str().unwrap_or(""));
                            if let Some(ext) = ext {
                                if let Ok(mut t_env) = tbl_env.try_borrow_mut() {
                                    match ext {
                                        "db" | "sqlite" | "sqlite3" => {
                                            t_env.try_backup(path);
                                        },
                                        _ => {
                                            if let Ok(mut f) = File::create(path) {
                                                let idx = tables_nb.get_page_index();
                                                if let Some(content) = t_env.get_text_at_index(idx, Some(settings)) {
                                                    let _ = f.write_all(&content.into_bytes());
                                                } else {
                                                    println!("Unable to get text at informed index");
                                                }
                                            } else {
                                                println!("Unable to create file");
                                            }
                                        }
                                    }
                                } else {
                                    println!("Unable to get reference to table environment");
                                }
                            } else {
                                println!("Unknown extension");
                            }
                        } else {
                            println!("No filename available");
                        }
                    },
                    _ => { }
                }
            });
        }

        {
            let dst = dst.clone();
            let save_tbl_dialog = save_tbl_dialog.clone();
            let settings = csv_window.settings.clone();
            let tables_nb = tables_nb.clone();
            let tbl_env = tbl_env.clone();
            let cmd_window = cmd_window.clone();
            save_tbl_btn.connect_clicked(move |_btn| {
                match *dst.borrow() {
                    Destination::File => {
                        save_tbl_dialog.run();
                        save_tbl_dialog.hide();
                    },
                    Destination::Clipboard => {
                        let settings = settings.borrow().clone();
                        let idx = tables_nb.get_page_index();
                        if let Ok(mut t_env) = tbl_env.try_borrow_mut() {
                            if let Some(content) = t_env.get_text_at_index(idx, Some(settings)) {
                                let opt_clip = gdk::Display::get_default()
                                    .and_then(|d| Clipboard::get_default(&d) );
                                if let Some(clip) = opt_clip {
                                    clip.set_text(&content);
                                    clip.store();
                                    println!("Current clipboard: {:?}", clip);
                                } else {
                                    println!("Unable to get default gdk display and/or clipboard");
                                }
                            } else {
                                println!("Unable to get text at informed index");
                            }
                        } else {
                            println!("Unable to get table index");
                        }
                    },
                    Destination::Program => {
                        cmd_window.set_expect_input(true);
                        if cmd_window.win.get_visible() {
                            cmd_window.win.grab_focus();
                        } else {
                            cmd_window.win.show();
                        }
                    }
                }
            });
        }
        
        Self {
            save_tbl_btn,
            save_tbl_dialog,
            tbl_format_combo,
            clipboard_toggle,
            program_toggle,
            file_toggle,
            dst,
            script_btn
        }
    }

}

// TODO use structure at sql::copy instead of this one
struct CopyAction {
    dst : String,
    cols : Vec<String>,
    convert : bool,
    create : bool
}

#[derive(Clone)]
pub struct CopyFromBox {
    db_table_entry : Entry,
    // col_subset_entry : Entry,
    // col_subset_check : CheckButton,
    create_check : CheckButton,
    convert_check : CheckButton,
    copy_from_btn : Button,
    script_from_btn : Button,
    action : Rc<RefCell<CopyAction>>,
}

impl CopyFromBox {

    pub fn build(
        builder : &Builder,
        tables_nb : &TableNotebook,
        tbl_env : &Rc<RefCell<TableEnvironment>>,
        csv_window : &CsvWindow,
        cmd_window : &CommandWindow
    ) -> Self {
        let action = Rc::new(RefCell::new(CopyAction{
            dst : String::new(),
            cols : Vec::new(),
            convert : false,
            create : false
        }));

        let db_table_entry : Entry = builder.get_object("db_table_entry").unwrap();
        {
            let action = action.clone();
            db_table_entry.connect_preedit_changed(move |_spin, txt| {
                action.borrow_mut().dst = txt.to_string();
            });
        }
        
        /* Column subset can be inferred from the CSV columns, read from program stdout or file.
        let col_subset_entry : Entry = builder.get_object("col_subset_entry").unwrap();
        {
            let action = action.clone();
            col_subset_entry.connect_preedit_changed(move |_spin, txt| {
                if txt.len() == 0 {
                    action.borrow_mut().cols.clear();
                } else {
                    let cols : Vec<_> = txt.split(',').map(|col| col.trim().to_string() ).collect();
                    println!("cols: {:?}", cols);
                    action.borrow_mut().cols = cols;
                }
            });
        }*/
        
        // let col_subset_check : CheckButton = builder.get_object("col_subset_check").unwrap();
        let create_check : CheckButton = builder.get_object("create_check").unwrap();
        let convert_check : CheckButton = builder.get_object("convert_check").unwrap();
        let copy_from_btn : Button = builder.get_object("copy_from_btn").unwrap();
        let script_from_btn : Button = builder.get_object("script_from_btn").unwrap();
        
        {
            let tables_nb = tables_nb.clone();
            let tbl_env = tbl_env.clone();
            let action = action.clone();
            copy_from_btn.connect_clicked(move |_btn| {
                let idx = tables_nb.get_page_index();
                if let Ok(mut t_env) = tbl_env.try_borrow_mut() {
                    if let Ok(action) = action.try_borrow() {
                        if let Err(e) = t_env.copy_to_database(idx, &action.dst[..], &action.cols[..]) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Unable to borrow action");
                    }
                } else {
                    println!("Unable to borrow table environment");
                }
            });
        }
        Self {
            db_table_entry,
            // col_subset_entry,
            // col_subset_check,
            create_check,
            convert_check,
            copy_from_btn,
            script_from_btn,
            action
        }
    }

}

#[derive(Clone)]
pub struct TablePopover {
    pub popover : Popover,
    // pub command_box : Box,
    // pub copy_box : Box,
    // pub upload_box : Box,
    // pub command_btn : Button,
    // pub upload_btn : Button,
    // finish_upload_btn : Button,
    // apply_btn : Button,
    // clear_btn : Button,

    table_size_label : Label,
    table_latency_label : Label,
    // backward_btn : Button,
    // forward_btn : Button,

    selected : Rc<RefCell<Option<usize>>>,
    
    copy_stack : Stack,
    copy_icon_stack : Stack,
    copy_to_bx : CopyToBox,
    copy_from_bx : CopyFromBox,
    csv_window : CsvWindow
}

impl TablePopover {

    pub fn set_copy_to(&self) {
        self.copy_stack.set_visible_child_name("copy_to");
        self.copy_icon_stack.set_visible_child_name("copy_to");
    }
    
    pub fn set_copy_from(&self) {
        self.copy_stack.set_visible_child_name("copy_from");
        self.copy_icon_stack.set_visible_child_name("copy_from");
    }
    
    pub fn show_at(&self, ev_box : &EventBox, ix : usize) {
        self.popover.hide();
        self.popover.set_relative_to(Some(ev_box));
        self.popover.show();
        self.selected.replace(Some(ix));
    }

    pub fn build(
        builder :  &Builder,
        workspace : PlotWorkspace,
        table_env : Rc<RefCell<TableEnvironment>>,
        tables_nb : TableNotebook,
        status_stack : StatusStack,
        cmd_window : CommandWindow
    ) -> Self {
        let popover : Popover = builder.get_object("table_popover").unwrap();
        // let command_box : Box = builder.get_object("command_box").unwrap();
        // let copy_box : Box = builder.get_object("table_copy_box").unwrap();
        let upload_box : Box = builder.get_object("table_upload_box").unwrap();

        let table_size_label : Label = builder.get_object("table_size_label").unwrap();
        let table_latency_label : Label = builder.get_object("table_latency_label").unwrap();
        // let forward_btn : Button = builder.get_object("table_forward_btn").unwrap();
        // let backward_btn : Button = builder.get_object("table_backward_btn").unwrap();

        // let upload_btn : Button = builder.get_object("upload_button").unwrap();
        // let finish_upload_btn : Button = builder.get_object("finish_upload_button").unwrap();

        let csv_window = CsvWindow::build(&builder);
        let copy_from_bx = CopyFromBox::build(&builder, &tables_nb, &table_env, &csv_window, &cmd_window);
        let copy_to_bx = CopyToBox::build(&builder, &tables_nb, &table_env, &csv_window, &cmd_window);
        let selected = Rc::new(RefCell::new(None));
        let copy_stack : Stack = builder.get_object("copy_stack").unwrap();
        let copy_icon_stack : Stack = builder.get_object("copy_icon_stack").unwrap();
        // let cmd_bx = CommandBox::new(&builder, &tables_nb, table_env.clone());
        Self {
            popover,
            // command_box,
            // command_btn,
            // apply_btn,
            // clear_btn,
            // command_entry,
            // copy_box,
            // upload_box,
            table_size_label,
            table_latency_label,
            // upload_btn,
            // finish_upload_btn,
            // forward_btn,
            // backward_btn,
            // cmd_bx,
            // save_bx,
            csv_window,
            selected,
            copy_stack,
            copy_icon_stack,
            copy_from_bx,
            copy_to_bx
        }
    }

}


