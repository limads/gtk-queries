use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;
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

