use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;
use gtk::*;
use crate::utils::RecentList;
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
use std::sync::mpsc::{channel, Sender, Receiver};
use std::process::Stdio;

pub struct Output {
    pub cmd : String,
    pub status : bool,
    pub txt : String
}

#[derive(Debug)]
pub struct Executor {
    // Command, with optional content to its standard input
    cmd_send : Sender<(String, Option<String>)>,
    ans_recv : Receiver<Output>
}

impl Executor {

    pub fn new() -> Self {
        let (cmd_send, cmd_recv) = channel::<(String, Option<String>)>();
        let (ans_send, ans_recv) = channel::<Output>();
        thread::spawn(move || {
            loop {
                if let Ok((cmd, tbl)) = cmd_recv.recv() {
                    match run_command(&cmd[..], tbl) {
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
        Self{ cmd_send, ans_recv }
    }

    pub fn queue_command(&self, cmd : String, tbl_csv : Option<String>) {
        match self.cmd_send.send((cmd, tbl_csv)) {
            Ok(_) => {
                // cmd_entry.set_sensitive(false);
                // clear_btn.set_sensitive(false);
                // run_btn.set_sensitive(false);
            },
            Err(e) => {
                println!("{}", e);
            }
        }
    }
    
    /// Blocks until a command is received, then executes the passed closure.
    pub fn wait_result<F>(&self, mut f : F) -> Result<(), String> 
    where
        F : FnMut(Output)->Result<(), String>
    {
        match self.ans_recv.recv() {
            Ok(out) => f(out),
            Err(e) => Err(format!("{}", e))
        }
    }
    
}

fn run_command(cmd : &str, opt_tbl : Option<String>) -> Result<String, String> {
    // TODO treat quoted arguments with whitespace as single units
    let split_cmd : Vec<_> = cmd.split(' ').collect();
    let cmd_name = split_cmd.get(0).ok_or(String::from("Command name missing"))?;
    let mut cmd = Command::new(&cmd_name)
        .args(split_cmd.get(1..).unwrap_or(&[]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}", e))?;
        
    if let Some(tbl) = opt_tbl {
        let mut outstdin = cmd.stdin.take().unwrap();
        let mut writer = BufWriter::new(&mut outstdin);
        writer.write_all(tbl.as_bytes()).map_err(|e| format!("{}", e))?;
    }
    
    // let mut stdout = cmd.stdout.take().ok_or(format!("Unable to read process stdout"))?;
    // let mut stderr = cmd.stderr.take().ok_or(format!("Unable to read process stderr"))?;
    
    let output = cmd.wait_with_output().map_err(|e| format!("{}", e))?;
    
    if output.status.success() {
        let mut stdout_content = String::from_utf8(output.stdout.clone())
            .map_err(|e| format!("{}", e))?;
        //    output.stdout.read_to_string(&mut stdout_content)
        //    .map_err(|e| format!("Error capturing stdout: {}", e))?;
        Ok(stdout_content)
    } else {
        //let mut stderr_content = String::new();
        // if let Err(e) = output.stderr.read_to_string(&mut stderr_content) {
        //    println!("{}", e);
        // }
        let mut stderr_content = String::from_utf8(output.stderr.clone())
            .map_err(|e| format!("{}", e))?;

        let code = output.status.code()
            .map(|code| code.to_string())
            .unwrap_or(String::from("Unknown"));
        Err(format!("Command error (Code: {}) {}", code, stderr_content))
    }
}

#[derive(Clone, Debug)]
pub struct CommandWindow {
    pub win : Window,
    cmd_list : ListBox,
    cmd_entry : Entry,
    clear_btn : Button,
    run_btn : Button,
    recent : RecentList
}

impl CommandWindow {

    // To run a shell-like string, we can pass the shell to stdin of /bin/sh like:
    // echo 'echo "hello"' | /bin/sh

    pub fn build(
        builder : &Builder, 
        table_notebook : &TableNotebook, 
        tbl_env : Rc<RefCell<TableEnvironment>>
    ) -> Self {
        let win : Window = builder.get_object("cmd_window").unwrap();
        let cmd_entry : Entry = builder.get_object("cmd_entry").unwrap();
        let run_btn : Button = builder.get_object("cmd_run_btn").unwrap();
        let clear_btn : Button = builder.get_object("cmd_clear_btn").unwrap();
        let recent = RecentList::new(Path::new("registry/commands.csv"), 11).unwrap();
        let cmd_list : ListBox = builder.get_object("cmd_list").unwrap();

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

        {
            let cmd_entry = cmd_entry.clone();
            let clear_btn = clear_btn.clone();
            let table_notebook = table_notebook.clone();
            run_btn.connect_clicked(move |run_btn| {
                let g_txt = cmd_entry.get_text();
                let txt = g_txt.as_str();
                if txt.len() >= 1  {
                    if let Ok(t_env) = tbl_env.try_borrow_mut() {
                        let ix = table_notebook.get_page_index();
                        if let Some(tbl_csv) = t_env.all_tables().get(ix).map(|tbl| tbl.to_csv() ) {    
                            // Moved to Executor::queue_command
                            /*match cmd_send.send((String::from(txt), tbl_csv)) {
                                Ok(_) => {
                                    cmd_entry.set_sensitive(false);
                                    clear_btn.set_sensitive(false);
                                    run_btn.set_sensitive(false);
                                },
                                Err(e) => {
                                    println!("{}", e);
                                }
                            }*/
                        } else {
                            println!("Invalid table index");
                        }
                    } else {
                        println!("Unable to borrow table");
                    }
                }
            });
        }

        let list = Self {
            win,
            cmd_entry,
            clear_btn,
            run_btn,
            recent,
            cmd_list
        };
        list
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
    
    /*fn wait_command() {
        // let cmd_bx = table_popover.cmd_bx.clone();
        let table_popover = table_popover.clone();
        let table_popover = table_popover.clone();
        glib::timeout_add_local(16, move || {
            /*if let Ok(out) = ans_recv.try_recv() {
                cmd_bx.cmd_entry.set_sensitive(true);
                cmd_bx.clear_btn.set_sensitive(true);
                cmd_bx.run_btn.set_sensitive(true);
                if out.status {
                    let add_res = utils::add_external_table(
                        &table_env,
                        &tables_nb,
                        out.command
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
            }*/
            glib::source::Continue(true)
        });
    }*/
}

