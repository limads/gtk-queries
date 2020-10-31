use gtk::prelude::*;
use std::env;
use gtk::*;
use crate::tables::environment::TableEnvironment;
use crate::table_notebook::*;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::table_popover::TablePopover;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::{Read, Write, Seek, SeekFrom};

#[derive(Debug)]
struct InnerList {
    f : File,
    lines : Vec<String>,
    mem : usize
}

/// Ref-counted list of strings that keeps its content synchronized with
/// a plain text file. This is used to save the list of commands executed
/// by the user and to save the list of last plot layout files opened by the user.
#[derive(Debug, Clone)]
pub struct RecentList(Rc<RefCell<InnerList>>);

impl RecentList {

    pub fn new(path : &Path, mem : usize) -> Option<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .append(false)
            .open(path)
            .ok()?;
        let recent = RecentList(Rc::new(RefCell::new(InnerList { f, lines : Vec::new(), mem })));
        recent.load_recent_paths();
        Some(recent)
    }

    fn write_to_file(lines : &[String], f : &mut File) {
        let mut path_file = String::new();
        for p in lines.iter() {
            path_file += &format!("{}\n", p)[..];
        }
        f.seek(SeekFrom::Start(0)).unwrap();
        if let Err(e) = f.write_all(&path_file.into_bytes()) {
            println!("Error writing to file: {}", e);
            return;
        }
        if let Err(e) = f.flush() {
            println!("{}", e);
            return;
        }
    }

    pub fn push_recent(&self, path : String) {
        if let Ok(mut t) = self.0.try_borrow_mut() {
            // println!("Current paths: {:?}", t.lines);
            // println!("New path: {:?}", path);
            if let Some(pos) = t.lines.iter().position(|p| &p[..] == &path[..]) {
                t.lines.remove(pos);
            }
            t.lines.push(path.clone());
            // println!("Path vector = {:?}", t.lines);
            if t.lines.len() >= t.mem {
                t.lines.remove(0);
            }
            let lines = t.lines.clone();
            Self::write_to_file(&lines[..], &mut t.f);
        } else {
            println!("Could not get mutable reference to recent layouts file for writing");
        }
    }

    pub fn remove(&self, txt : &str) {
        if let Ok(mut t) = self.0.try_borrow_mut() {
            if let Some(ix) = t.lines.iter().position(|l| &l[..] == txt) {
                t.lines.remove(ix);
                let lines = t.lines.clone();
                Self::write_to_file(&lines[..], &mut t.f);
            } else {
                println!("Entry {} not available to be removed", txt);
            }
        } else {
            println!("Unable to borrow recent path");
        }
    }

    pub fn load_recent_paths(&self) {
        if let Ok(mut t) = self.0.try_borrow_mut() {
            let mut content = String::new();
            if let Ok(_) = t.f.read_to_string(&mut content) {
                t.lines.clear();
                t.lines.extend(content.lines().map(|l| {
                    println!("line: {:?}", l); l.to_string()
                }));
            } else {
                println!("Failed reading path sequence file");
            }
        } else {
            println!("Failed acquiring reference to recent files");
        }
    }

    pub fn loaded_items(&self) -> Vec<String> {
        if let Ok(t) = self.0.try_borrow() {
            t.lines.clone()
        } else {
            println!("Unable to borrow recent list");
            Vec::new()
        }
    }

}

pub fn link_window<B>(btn : B, win : Window)
where
    B : IsA<Button> + ButtonExt
{
    {
        let win = win.clone();
        btn.connect_clicked(move |_| {
            if win.get_visible() {
                win.grab_focus();
            } else {
                win.show();
            }
        });
    }
    win.set_destroy_with_parent(false);
    win.connect_delete_event(move |win, ev| {
        win.hide();
        glib::signal::Inhibit(true)
    });
    win.connect_destroy_event(move |win, ev| {
        win.hide();
        glib::signal::Inhibit(true)
    });
}

/// Add tables resulting from a query sequence.
pub fn set_tables(
    table_env : &TableEnvironment,
    tables_nb : &mut TableNotebook,
    // mapping_popover : Popover,
    workspace : PlotWorkspace,
    table_popover : TablePopover
    //fn_popover : Popover
) {
    tables_nb.clear();
    let all_tbls = table_env.all_tables_as_rows();
    if all_tbls.len() == 0 {
        tables_nb.add_page(
            "application-exit",
            None,
            Some("No queries"),
            None,
            // mapping_popover.clone(),
            workspace.clone(),
            table_popover.clone()
            //fn_popover.clone()
        );
    } else {
        tables_nb.clear();
        for t_rows in all_tbls {
            let nrows = t_rows.len();
            // println!("New table with {} rows", nrows);
            if nrows > 0 {
                let ncols = t_rows[0].len();
                let name = format!("({} x {})", nrows - 1, ncols);
                tables_nb.add_page(
                    "network-server-symbolic",
                    Some(&name[..]),
                    None,
                    Some(t_rows),
                    // mapping_popover.clone(),
                    workspace.clone(),
                    table_popover.clone()
                    //fn_popover.clone()
                );
            } else {
                println!("No rows to display");
            }
        }
    }
}

fn exec_dir() -> Result<String, &'static str> {
    let exe_path = env::current_exe().map_err(|_| "Could not get executable path")?;
    let exe_dir = exe_path.as_path().parent().ok_or("CLI executable has no parent dir")?
        .to_str().ok_or("Could not convert path to str")?;
    Ok(exe_dir.to_string())
}

pub fn glade_path(filename : &str) -> Result<String, &'static str> {
    let exe_dir = exec_dir()?;
    let path = exe_dir + "/../../assets/gui/" + filename;
    Ok(path)
}

pub fn provider_from_path(filename : &str) -> Result<CssProvider, &'static str> {
    let provider =  CssProvider::new();
    let exe_dir = exec_dir()?;
    let path = exe_dir + "/../../assets/styles/" + filename;
    println!("{}", path);
    provider.load_from_path(&path[..]).map_err(|_| "Unable to load Css provider")?;
    Ok(provider)
}

pub fn show_popover_on_toggle(popover : &Popover, toggle : &ToggleButton) {
    {
        let popover = popover.clone();
        toggle.connect_toggled(move |btn| {
            if btn.get_active() {
                popover.show();
            } else {
                popover.hide();
            }
        });
    }

    {
        let toggle = toggle.clone();
        popover.connect_closed(move |_| {
            if toggle.get_active() {
                toggle.set_active(false);
            }
        });
    }
}

