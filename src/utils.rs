use gtk::prelude::*;
use std::env;
use gtk::*;
use crate::tables::environment::TableEnvironment;
use crate::table_notebook::*;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::table_popover::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::{Read, Write, Seek, SeekFrom};
use crate::tables::table::Table;
use crate::status_stack::{Status, StatusStack};
use glib::{types::Type, value::{Value, ToValue}};
use gdk_pixbuf::Pixbuf;

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
        let recent = RecentList(Rc::new(RefCell::new(InnerList {
            f,
            lines : Vec::new(),
            mem
        })));
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
            if let Some(pos) = t.lines.iter().position(|p| &p[..] == &path[..]) {
                t.lines.remove(pos);
            }
            t.lines.push(path.clone());
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
    win.connect_delete_event(move |win, _ev| {
        win.hide();
        glib::signal::Inhibit(true)
    });
    win.connect_destroy_event(move |win, _ev| {
        win.hide();
        glib::signal::Inhibit(true)
    });
}

/// Add tables resulting from a query sequence.
pub fn set_tables_from_query(
    table_env : &TableEnvironment,
    tables_nb : &mut TableNotebook,
    workspace : PlotWorkspace,
    table_bar : TableBar
) {
    tables_nb.clear();
    let all_tbls = table_env.all_tables();
    if all_tbls.len() == 0 {
        tables_nb.create_error_table("No tables to show");
    } else {
        tables_nb.clear();
        table_bar.set_copy_to();
        for table in all_tbls.iter() {
            let info = table.table_info();
            tables_nb.create_data_table(
                TableSource::Database(info.0, info.1),
                table.text_rows(),
                workspace.clone(),
                table_bar.clone()
            );
        }
    }
}

/// Add an external table to the environment/notebook,
/// parseable from the CSV at the txt argument
pub fn add_external_table(
    table_env : &Rc<RefCell<TableEnvironment>>,
    tables_nb : &TableNotebook,
    source : TableSource,
    txt : String,
    workspace : &PlotWorkspace,
    table_bar : &TableBar,
    status_stack : &StatusStack
) -> Result<(), String> {
    match Table::new_from_text(txt) {
        Ok(tbl) => {
            let rows = tbl.text_rows();
            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                if let Err(e) = t_env.append_external_table(tbl) {
                    Err(format!("Error appending table: {}", e))?;
                }
            } else {
                Err(format!("Unable to borrow table environment"))?;
            }
            // If external table is opened by file, name as file name, without the extension,
            // and use blank page as icon.
            // If external table is opened by program, use Std. Output (progname) as name,
            // and use bash-symbolic as icon.
            table_bar.set_copy_from();
            tables_nb.create_data_table(source, rows, workspace.clone(), table_bar.clone() );
            status_stack.update(Status::Ok);
            Ok(())
        },
        Err(e) => {
            status_stack.update(Status::SqlErr(e.into()));
            Err(format!("Error parsing table: {}", e))
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
    provider.load_from_path(&path[..]).map_err(|_| "Unable to load CSS provider")?;
    Ok(provider)
}

pub fn show_popover_on_toggle(popover : &Popover, toggle : &ToggleButton, alt : Vec<ToggleButton>) {
    {
        let popover = popover.clone();
        toggle.connect_toggled(move |btn| {
            if btn.get_active() {
                popover.show();
                for toggle in alt.iter() {
                    if toggle.get_active() {
                        toggle.set_active(false);
                    }
                }
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

pub fn break_string(content : &mut String, line_length : usize) {
    let mut break_next = false;
    for i in 1..content.len() {
        if i % line_length == 0  {
            break_next = true;
        }
        if break_next && content.chars().nth(i) == Some(' ') {
            content.replace_range(i..i+1, "\n");
            break_next = false;
        }
    }
}

pub fn configure_tree_view(tree_view : &TreeView) -> TreeStore {
    let model = TreeStore::new(&[Pixbuf::static_type(), Type::String]);
    tree_view.set_model(Some(&model));
    let pix_renderer = CellRendererPixbuf::new();
    pix_renderer.set_property_height(24);
    let txt_renderer = CellRendererText::new();
    txt_renderer.set_property_height(24);

    let pix_col = TreeViewColumn::new();
    pix_col.pack_start(&pix_renderer, false);
    pix_col.add_attribute(&pix_renderer, "pixbuf", 0);

    let txt_col = TreeViewColumn::new();
    txt_col.pack_start(&txt_renderer, true);
    txt_col.add_attribute(&txt_renderer, "text", 1);

    tree_view.append_column(&pix_col);
    tree_view.append_column(&txt_col);
    tree_view.set_show_expanders(true);
    model
}
