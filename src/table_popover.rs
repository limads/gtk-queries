use gdk;
use gtk::*;
// use gio::prelude::*;
// use std::env::args;
use std::rc::Rc;
use std::cell::RefCell;
// use std::fs::File;
// use std::io::Write;
// use sourceview::*;
use gtk::prelude::*;
// use crate::{utils, table_notebook::TableNotebook };
use crate::utils::RecentList;
use std::sync::mpsc::{channel, /*Sender,*/ Receiver};
use std::thread;
use std::process::Command;
use std::path::Path;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::tables::environment::TableEnvironment;
use crate::table_notebook::TableNotebook;
use glib;
// use crate::tables::table::Table;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use crate::tables::table::{Format, TableSettings, NullField, BoolField, Align};
use std::default::Default;
use crate::utils;
use crate::status_stack::StatusStack;

struct Output {
    cmd : String,
    status : bool,
    txt : String
}

#[derive(Clone)]
struct CommandBox {
    cmd_list : ListBox,
    cmd_entry : Entry,
    clear_btn : Button,
    run_btn : Button,
    recent : RecentList
}

impl CommandBox {

    fn run_command(cmd : &str) -> Result<String, String> {
        let split_cmd : Vec<_> = cmd.split(' ').collect();
        let cmd_name = split_cmd.get(0).ok_or(String::from("Command name missing"))?;
        let output = Command::new(&cmd_name)
            .args(split_cmd.get(1..).unwrap_or(&[]))
            .output()
            .map_err(|e| format!("{}", e))?;
        let status = output.status;
        let stderr : Option<String> = String::from_utf8(output.stderr).ok();
        if status.success() {
            if status.code() == Some(0) {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    Ok(stdout)
                } else {
                    Err(format!("Unable to parse stdout"))
                }
            } else {
                Err(format!("Command error ({:?}): {}", status.code(), stderr.unwrap_or(String::new())))
            }
        } else {
            Err(format!("{}", stderr.unwrap_or(String::new())))
        }
    }

    fn new(builder : &Builder) -> (Self, Receiver<Output>) {
        let cmd_entry : Entry = builder.get_object("cmd_entry").unwrap();
        let run_btn : Button = builder.get_object("cmd_run_btn").unwrap();
        let clear_btn : Button = builder.get_object("cmd_clear_btn").unwrap();
        let recent = RecentList::new(Path::new("registry/commands.csv"), 11).unwrap();
        let cmd_list : ListBox = builder.get_object("cmd_list").unwrap();

        let (cmd_send, cmd_recv) = channel::<String>();
        let (ans_send, ans_recv) = channel::<Output>();

        {
            let cmd_entry = cmd_entry.clone();
            let recent = recent.clone();
            clear_btn.connect_clicked(move |_| {
                let g_txt = cmd_entry.get_text();
                let txt = g_txt.as_str();
                if txt.len() >= 1 {
                    recent.remove(txt);
                }
                cmd_entry.set_text("");
            });
        }

        thread::spawn(move || {
            loop {
                if let Ok(cmd) = cmd_recv.recv() {
                    match Self::run_command(&cmd[..]) {
                        Ok(txt) => {
                            if let Err(e) = ans_send.send(Output { cmd, status : true, txt }) {
                                println!("{}", e);
                            }
                        },
                        Err(txt) => {
                            if let Err(e) = ans_send.send(Output { cmd, status : false, txt }) {
                                println!("{}", e);
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });

        {
            let cmd_entry = cmd_entry.clone();
            let clear_btn = clear_btn.clone();
            run_btn.connect_clicked(move |run_btn| {
                let g_txt = cmd_entry.get_text();
                let txt = g_txt.as_str();
                if txt.len() >= 1  {
                    match cmd_send.send(String::from(txt)) {
                        Ok(_) => {
                            cmd_entry.set_sensitive(false);
                            clear_btn.set_sensitive(false);
                            run_btn.set_sensitive(false);
                        },
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                }
            });
        }

        let list = Self {
            cmd_entry,
            clear_btn,
            run_btn,
            recent,
            cmd_list
        };
        (list, ans_recv)
    }

    fn update_commands(
        recent : &RecentList,
        cmd_list : &ListBox,
    ) {
        for child in cmd_list.get_children() {
            cmd_list.remove(&child);
        }
        for (i, cmd) in recent.loaded_items().iter().enumerate() {
            let lbl_cmd = Label::new(Some(cmd));
            lbl_cmd.set_property_width_request(120);
            lbl_cmd.set_property_width_request(120);
            let row = ListBoxRow::new();
            row.add(&lbl_cmd);
            row.set_selectable(true);
            row.set_property_height_request(36);
            cmd_list.insert(&row, i as i32);
        }
        cmd_list.show_all();
    }
}

#[derive(Clone)]
struct SaveTblBox {
    save_tbl_btn : Button,
    tbl_format_combo : ComboBoxText,
    save_tbl_dialog : FileChooserDialog,
    settings : Rc<RefCell<TableSettings>>,
    tbl_bool_combo : ComboBoxText,
    tbl_null_combo : ComboBoxText,
    tbl_prec_spin : SpinButton,
    align_left_radio : RadioButton,
    align_center_radio : RadioButton,
    align_right_radio : RadioButton,
    clipboard_tbl_btn : Button
}

impl SaveTblBox {

    fn build(
        builder : &Builder,
        tables_nb : &TableNotebook,
        tbl_env : &Rc<RefCell<TableEnvironment>>
    ) -> Self {

        let settings : Rc<RefCell<TableSettings>> = Rc::new(RefCell::new(Default::default()));
        let tbl_format_combo : ComboBoxText = builder.get_object("tbl_format_combo").unwrap();
        {
            let settings = settings.clone();
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

        let save_tbl_dialog : FileChooserDialog =
            builder.get_object("save_tbl_dialog").unwrap();
        {
            let settings = settings.clone();
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

        let save_tbl_btn : Button =
            builder.get_object("save_tbl_btn").unwrap();
        {
            let save_tbl_dialog = save_tbl_dialog.clone();
            save_tbl_btn.connect_clicked(move |_btn| {
                save_tbl_dialog.run();
                save_tbl_dialog.hide();
            });
        }

        let clipboard_tbl_btn : Button =
            builder.get_object("clipboard_tbl_btn").unwrap();
        {
            let settings = settings.clone();
            let tables_nb = tables_nb.clone();
            let tbl_env = tbl_env.clone();
            clipboard_tbl_btn.connect_clicked(move |_btn| {
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
            });
        }
        Self {
            save_tbl_btn,
            save_tbl_dialog,
            tbl_bool_combo,
            tbl_null_combo,
            tbl_prec_spin,
            align_left_radio,
            align_center_radio,
            align_right_radio,
            tbl_format_combo,
            settings,
            clipboard_tbl_btn
        }
    }

}

struct CopyAction {
    dst : String,
    cols : Vec<String>,
    strict : bool,
    create : bool
}

#[derive(Clone)]
pub struct CopyBox {
    db_table_entry : Entry,
    col_subset_entry : Entry,
    col_subset_check : CheckButton,
    create_check : CheckButton,
    strict_check : CheckButton,
    copy_db_btn : Button,
    action : Rc<RefCell<CopyAction>>
}

impl CopyBox {

    pub fn build(
        builder : &Builder,
        tables_nb : &TableNotebook,
        tbl_env : &Rc<RefCell<TableEnvironment>>
    ) -> Self {
        let action = Rc::new(RefCell::new(CopyAction{
            dst : String::new(),
            cols : Vec::new(),
            strict : false,
            create : false
        }));

        let db_table_entry : Entry = builder.get_object("db_table_entry").unwrap();
        {
            let action = action.clone();
            db_table_entry.connect_preedit_changed(move |_spin, txt| {
                action.borrow_mut().dst = txt.to_string();
            });
        }

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
        }
        let col_subset_check : CheckButton = builder.get_object("col_subset_check").unwrap();
        let create_check : CheckButton = builder.get_object("create_check").unwrap();
        let strict_check : CheckButton = builder.get_object("strict_check").unwrap();
        let copy_db_btn : Button = builder.get_object("copy_db_btn").unwrap();
        {
            let tables_nb = tables_nb.clone();
            let tbl_env = tbl_env.clone();
            let action = action.clone();
            copy_db_btn.connect_clicked(move |_btn| {
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
            col_subset_entry,
            col_subset_check,
            create_check,
            strict_check,
            copy_db_btn,
            action
        }
    }

}

#[derive(Clone)]
pub struct TablePopover {
    pub popover : Popover,
    // pub command_box : Box,
    // pub copy_box : Box,
    pub upload_box : Box,
    // pub command_btn : Button,
    // pub upload_btn : Button,
    // finish_upload_btn : Button,
    // apply_btn : Button,
    // clear_btn : Button,

    table_size_label : Label,
    table_latency_label : Label,
    backward_btn : Button,
    forward_btn : Button,

    selected : Rc<RefCell<Option<usize>>>,
    cmd_bx : CommandBox,
    save_bx : SaveTblBox,
    copy_bx : CopyBox
}

impl TablePopover {

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
        status_stack : StatusStack
    ) -> Self {
        let popover : Popover = builder.get_object("table_popover").unwrap();
        // let command_box : Box = builder.get_object("command_box").unwrap();
        // let copy_box : Box = builder.get_object("table_copy_box").unwrap();
        let upload_box : Box = builder.get_object("table_upload_box").unwrap();

        let table_size_label : Label = builder.get_object("table_size_label").unwrap();
        let table_latency_label : Label = builder.get_object("table_latency_label").unwrap();
        let forward_btn : Button = builder.get_object("table_forward_btn").unwrap();
        let backward_btn : Button = builder.get_object("table_backward_btn").unwrap();

        // let upload_btn : Button = builder.get_object("upload_button").unwrap();
        // let finish_upload_btn : Button = builder.get_object("finish_upload_button").unwrap();

        let save_bx = SaveTblBox::build(&builder, &tables_nb, &table_env);
        let (cmd_bx, ans_recv) = CommandBox::new(&builder);
        let copy_bx = CopyBox::build(&builder, &tables_nb, &table_env);
        let selected = Rc::new(RefCell::new(None));
        let table_popover = Self {
            popover,
            // command_box,
            // command_btn,
            // apply_btn,
            // clear_btn,
            // command_entry,
            // copy_box,
            upload_box,
            table_size_label,
            table_latency_label,
            // upload_btn,
            // finish_upload_btn,
            forward_btn,
            backward_btn,
            cmd_bx,
            save_bx,
            selected,
            copy_bx
        };

        {
            let cmd_bx = table_popover.cmd_bx.clone();
            let table_popover = table_popover.clone();
            let table_popover = table_popover.clone();
            glib::timeout_add_local(16, move || {
                if let Ok(out) = ans_recv.try_recv() {
                    cmd_bx.cmd_entry.set_sensitive(true);
                    cmd_bx.clear_btn.set_sensitive(true);
                    cmd_bx.run_btn.set_sensitive(true);
                    if out.status {
                        let add_res = utils::add_external_table(
                            &table_env,
                            &tables_nb,
                            out.txt.clone(),
                            &workspace,
                            &table_popover,
                            &status_stack
                        );
                        match add_res {
                            Ok(_) => {
                                cmd_bx.recent.push_recent(out.cmd.clone());
                                CommandBox::update_commands(&cmd_bx.recent, &cmd_bx.cmd_list);
                            },
                            Err(e) => {
                                println!("{}", e);
                                tables_nb.add_page(
                                    "bash-symbolic",
                                    None,
                                    Some(&e[..]),
                                    None,
                                    workspace.clone(),
                                    table_popover.clone()
                                );
                            }
                        }
                    } else {
                        tables_nb.add_page(
                            "bash-symbolic",
                            None,
                            Some(&format!("Command output: {}",&out.txt)[..]),
                            None,
                            workspace.clone(),
                            table_popover.clone()
                        );
                    }
                }
                glib::source::Continue(true)
            });
        }

        table_popover
    }

}


