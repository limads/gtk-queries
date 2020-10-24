// #![recursion_limit = "512"]

pub mod table_widget;

pub mod table_notebook;

pub mod status_stack;

pub mod sql_editor;

pub mod functions;

pub mod conn_popover;

pub mod plots;

pub mod upload_popover;

pub mod tables;

pub mod query_sidebar;

pub mod main_menu;

pub mod file_list;

pub mod schema_tree;

pub mod jobs;

pub mod table_popover;

pub mod utils {

    use gtk::prelude::*;
    // use gio::prelude::*;
    use std::env;
    use gtk::*;
    use crate::tables::environment::TableEnvironment;
    use crate::table_notebook::*;
    use crate::plots::plot_workspace::PlotWorkspace;
    use crate::table_popover::TablePopover;

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

    pub fn set_tables(
        table_env : &TableEnvironment,
        tables_nb : &mut TableNotebook,
        mapping_popover : Popover,
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
                mapping_popover.clone(),
                workspace.clone(),
                table_popover.clone()
                //fn_popover.clone()
            );
        } else {
            tables_nb.clear();
            for t_rows in all_tbls {
                let nrows = t_rows.len();
                //println!("New table with {} rows", nrows);
                if nrows > 0 {
                    let ncols = t_rows[0].len();
                    let name = format!("({} x {})", nrows - 1, ncols);
                    tables_nb.add_page(
                        "network-server-symbolic",
                        Some(&name[..]),
                        None,
                        Some(t_rows),
                        mapping_popover.clone(),
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

}


