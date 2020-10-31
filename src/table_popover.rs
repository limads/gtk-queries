use gtk::*;
use gio::prelude::*;
// use std::env::args;
use std::rc::Rc;
use std::cell::RefCell;
// use std::fs::File;
// use std::io::Write;
use gdk::{self, keys};
use sourceview::*;
use gtk::prelude::*;
// use crate::{utils, table_notebook::TableNotebook };
use crate::utils::RecentList;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::process::Command;
use std::convert::TryFrom;
use std::path::Path;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::tables::environment::TableEnvironment;
use crate::table_notebook::TableNotebook;
use glib;
use crate::tables::table::Table;

struct Output {
    cmd : String,
    status : bool,
    txt : String
}

#[derive(Clone)]
struct CommandList {
    cmd_list : ListBox,
    cmd_entry : Entry,
    clear_btn : Button,
    run_btn : Button,
    recent : RecentList
}

impl CommandList {

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
                    if let Err(e) = cmd_send.send(String::from(txt)) {
                        println!("{}", e);
                    }
                }
                cmd_entry.set_sensitive(false);
                clear_btn.set_sensitive(false);
                run_btn.set_sensitive(false);
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
pub struct TablePopover {
    pub popover : Popover,
    // pub command_box : Box,
    //pub copy_box : Box,
    pub upload_box : Box,
    // pub command_btn : Button,
    // pub upload_btn : Button,
    finish_upload_btn : Button,
    apply_btn : Button,
    clear_btn : Button,

    table_size_label : Label,
    table_latency_label : Label,
    backward_btn : Button,
    forward_btn : Button,
    cmd_list : CommandList
}

impl TablePopover {

    pub fn build(
        builder :  &Builder,
        workspace : PlotWorkspace,
        table_env : Rc<RefCell<TableEnvironment>>,
        tables_nb : TableNotebook
    ) -> Self {
        let popover : Popover = builder.get_object("table_popover").unwrap();
        // let command_box : Box = builder.get_object("command_box").unwrap();
        //let copy_box : Box = builder.get_object("table_copy_box").unwrap();
        let upload_box : Box = builder.get_object("table_upload_box").unwrap();

        let table_size_label : Label = builder.get_object("table_size_label").unwrap();
        let table_latency_label : Label = builder.get_object("table_latency_label").unwrap();
        let forward_btn : Button = builder.get_object("table_forward_btn").unwrap();
        let backward_btn : Button = builder.get_object("table_backward_btn").unwrap();

        //let upload_btn : Button = builder.get_object("upload_button").unwrap();
        let finish_upload_btn : Button = builder.get_object("finish_upload_button").unwrap();

        let apply_btn : Button = builder.get_object("command_apply_btn").unwrap();
        let clear_btn : Button = builder.get_object("command_clear_btn").unwrap();

        let (cmd_list, ans_recv) = CommandList::new(&builder);
        let table_popover = Self {
            popover,
            // command_box,
            // command_btn,
            apply_btn,
            clear_btn,
            //command_entry,
            // copy_box,
            upload_box,
            table_size_label,
            table_latency_label,
            // upload_btn,
            finish_upload_btn,
            forward_btn,
            backward_btn,
            cmd_list
        };

        {
            let cmd_list = table_popover.cmd_list.clone();
            let table_popover = table_popover.clone();
            glib::timeout_add_local(16, move || {
                if let Ok(out) = ans_recv.try_recv() {
                    cmd_list.cmd_entry.set_sensitive(true);
                    cmd_list.clear_btn.set_sensitive(true);
                    cmd_list.run_btn.set_sensitive(true);
                    if out.status {
                        if let Ok(tbl) = Table::new_from_text(out.txt.clone()) {
                            let rows = tbl.text_rows();
                            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                                if let Err(e) = t_env.append_external_table(tbl) {
                                    println!("Error appending table: {}", e);
                                }
                            } else {
                                println!("Unable to borrow table environment");
                                return glib::source::Continue(true);
                            }
                            tables_nb.add_page(
                                "bash-symbolic",
                                Some("Std. Output (1)"),
                                None,
                                Some(rows),
                                workspace.clone(),
                                table_popover.clone()
                            );
                            cmd_list.recent.push_recent(out.cmd.clone());
                            CommandList::update_commands(&cmd_list.recent, &cmd_list.cmd_list);
                        } else {
                            tables_nb.add_page(
                                "bash-symbolic",
                                None,
                                Some(&format!("Command output: {}", out.txt.clone())),
                                None,
                                workspace.clone(),
                                table_popover.clone()
                            );
                        }
                    } else {
                        tables_nb.add_page(
                            "bash-symbolic",
                            None,
                            Some(&out.txt),
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


