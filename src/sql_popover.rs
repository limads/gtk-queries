use gtk::*;
use gio::prelude::*;
use std::env::{self, args};
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::collections::HashMap;
// use gtk_plots::conn_popover::{ConnPopover, TableDataSource};
use std::path::PathBuf;
// use sourceview::*;
use std::ffi::OsStr;
use gdk::ModifierType;
use gdk::{self, enums::key};
use tables::{self, environment_source::EnvironmentSource, TableEnvironment, button::TableChooser, sql::SqlListener};
use sourceview::*;
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use crate::{utils, table_widget::TableWidget, table_notebook::TableNotebook, status_stack::StatusStack };
use nlearn::table::Table;
use crate::status_stack::*;
use sourceview::View;

pub enum ExecStatus {
    File(String, usize),
    View(String, usize),
    None
}

#[derive(Clone)]
pub struct SqlPopover {
    pub view : View,
    //pub sql_load_dialog : FileChooserDialog,
    pub refresh_btn : ToolButton,
    pub popover : Popover,
    pub query_toggle : ToggleButton,
    //pub sql_toggle : ToggleToolButton,
    pub file_loaded : Rc<RefCell<bool>>,
    pub query_sent : Rc<RefCell<bool>>,
    pub extra_toolbar : Toolbar,
    pub sql_stack : Stack,
    pub status_stack : StatusStack,
    t_env : Rc<RefCell<TableEnvironment>>
}

impl SqlPopover {

    pub fn update_queries(
        file_loaded : Rc<RefCell<bool>>,
        query_sent : Rc<RefCell<bool>>,
        tbl_env : &mut TableEnvironment,
        view : &sourceview::View,
        //nb : &TableNotebook
    ) -> Result<(), &'static str> {
        if let Ok(loaded) = file_loaded.try_borrow() {
            if *loaded {
                tbl_env.send_current_query();
                //nb.nb.set_sensitive(false);
            } else {
                if let Some(buffer) = view.get_buffer() {
                    let text : Option<String> = match buffer.get_selection_bounds() {
                        Some((from,to,)) => {
                            from.get_text(&to).map(|txt| txt.to_string())
                        },
                        None => {
                            buffer.get_text(
                                &buffer.get_start_iter(),
                                &buffer.get_end_iter(),
                                true
                            ).map(|txt| txt.to_string())
                        }
                    };
                    if let Some(txt) = text {
                        //println!("{}", txt);
                        tbl_env.prepare_and_send_query(txt);
                        view.set_sensitive(false);
                        //nb.nb.set_sensitive(false);
                    } else {
                        println!("No text available to send");
                    }
                } else {
                    println!("Could not retrieve text buffer");
                    return Err("Error");
                }
            }
        } else {
            println!("Could not retrieve reference to file status");
            return Err("Error");
        }
        if let Ok(mut sent) = query_sent.try_borrow_mut() {
            *sent = true;
            //println!("Query sent");
            Ok(())
        } else {
            println!("Unable to acquire lock over query sent status");
            Err("Error")
        }
        //println!("at update: {}", query_sent.borrow());
    }

    pub fn add_extra_toolbar(&self, tool_btn : ToggleToolButton, bx : gtk::Box) {
        self.sql_stack.add_named(&bx, "extra");
        let sql_stack = self.sql_stack.clone();
        tool_btn.connect_toggled(move|item| {
            if item.get_active() {
                sql_stack.set_visible_child_name("extra");
            } else {
                sql_stack.set_visible_child_name("source");
            }
        });
        self.extra_toolbar.insert(&tool_btn, 0);
        self.extra_toolbar.show_all();
    }

    pub fn connect_result_arrived<F>(
        &self,
        //tbl_env_c : Rc<RefCell<TableEnvironment>>,
        mut f : F
    )
        where
            F : FnMut(&TableEnvironment) -> Result<(), String> + 'static
    {
        let tbl_env_c = self.t_env.clone();
        let status_stack = self.status_stack.clone();
        let view_c = self.view.clone();
        let sql_popover = self.clone();
        gtk::timeout_add(16, move || {
            if let Ok(mut sent) = sql_popover.query_sent.try_borrow_mut() {
                if *sent {
                    println!("Sent");
                    //println!("{}", sql_popover.query_sent.borrow());
                    if let Ok(mut t_env) = tbl_env_c.try_borrow_mut() {
                        let updated = if let Some(last_cmd) = t_env.last_commands().last() {
                            println!("Query updated");
                            println!("Last command: {}", last_cmd);
                            if &last_cmd[..] == "select" {
                                match t_env.maybe_update_from_query_results() {
                                    Some(ans) => {
                                        match ans {
                                            Ok(_) => {
                                                if let Err(e) = f(&t_env) {
                                                    println!("{}", e);
                                                    status_stack.update(Status::SqlErr(e));
                                                } else {
                                                    status_stack.update(Status::Ok);
                                                }
                                            },
                                            Err(e) => {
                                                println!("{}", e);
                                                status_stack.update(Status::SqlErr(e));
                                            }
                                        }
                                        true
                                    },
                                    None => false
                                }
                            } else {
                                match t_env.result_last_statement() {
                                    Some(ans) => {
                                        match ans {
                                            Ok(msg) => {
                                                status_stack.update(Status::StatementExecuted(msg));
                                            },
                                            Err(e) => {
                                                println!("{}", e);
                                                status_stack.update(Status::SqlErr(e));
                                            }
                                        }
                                        true
                                    },
                                    None => false
                                }
                            }
                        } else {
                            println!("Unable to retrieve last command");
                            false
                        };
                        if updated {
                            view_c.set_sensitive(true);
                            *sent = false;
                            println!("Sent set to false");
                        } else {
                            println!("Not updated yet");
                        }
                    }
                }
            } else {
                println!("Unable to retrieve reference to query sent status");
            }
            glib::source::Continue(true)
        });
    }

    pub fn new(query_toggle : ToggleButton, status_stack : StatusStack, t_env : Rc<RefCell<TableEnvironment>>) -> Self {
        let query_popover_path = utils::glade_path("query-popover.glade").expect("Failed to load glade file");
        let builder = Builder::new_from_file(query_popover_path);
        let popover : Popover =
            builder.get_object("query_popover").unwrap();
        let view : View =
            builder.get_object("query_source").unwrap();
        let buffer = view.get_buffer().unwrap()
            .downcast::<sourceview::Buffer>().unwrap();
        let lang_manager = LanguageManager::get_default().unwrap();
        let lang = lang_manager.get_language("sql").unwrap();
        buffer.set_language(Some(&lang));
        let sql_stack : Stack = builder.get_object("sql_stack").unwrap();
        let extra_toolbar : Toolbar = builder.get_object("extra_toolbar").unwrap();
        let sql_toolbar : Toolbar = builder.get_object("sql_toolbar").unwrap();
        let img_clock = Image::new_from_icon_name(Some("clock-app-symbolic"), IconSize::SmallToolbar);
        let update_btn = ToggleToolButton::new();
        update_btn.set_icon_widget(Some(&img_clock));
        sql_toolbar.insert(&update_btn, 1);
        // update_toolbar.insert(&item_b, 1);
        // extra_toolbar.show_all();
        update_btn.connect_toggled(move|btn|{
            /*let curr_label = btn.get_label().unwrap();
            let new_label = match curr_label.as_str() {
                "Off" => "0.5 s",
                "0.5 s" => "1 s",
                "1 s" => "5 s",
                "5 s" => "Off",
                _ => "Off"
            };
            btn.set_label(Some(new_label));*/
        });
        //let sql_load_dialog : FileChooserDialog =
        //    builder.get_object("sql_load_dialog").unwrap();
        /*{
            let view = view.clone();

            {
                let sql_load_dialog = sql_load_dialog.clone();
                sql_toggle.connect_toggled(move|item| {
                    if item.get_active() {
                        sql_load_dialog.run();
                        sql_load_dialog.hide();
                    } else {
                        item.set_label(None);
                        view.set_sensitive(true);
                    }
                });
            }
        }*/

        //let exec_toolbar : Toolbar = builder.get_object("exec_toolbar").unwrap();
        let img_refresh = Image::new_from_icon_name(Some("view-refresh"), IconSize::SmallToolbar);
        let refresh_btn : ToolButton = ToolButton::new(Some(&img_refresh), None);
        sql_toolbar.insert(&refresh_btn, 0);
        sql_toolbar.show_all();

        {
            let popover = popover.clone();
            query_toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    popover.show();
                } else {
                    popover.hide();
                }
            });
        }

        {
            let query_toggle = query_toggle.clone();
            popover.connect_closed(move |_popover| {
                query_toggle.set_active(false);
            });
        }

        popover.set_relative_to(Some(&query_toggle));

        Self {
            view,
            //sql_load_dialog,
            refresh_btn,
            popover,
            query_toggle,
            extra_toolbar,
            // sql_toggle,
            file_loaded : Rc::new(RefCell::new(false)),
            query_sent : Rc::new(RefCell::new(false)),
            sql_stack,
            status_stack,
            t_env
        }
    }

    pub fn set_file_mode(&self, fname : &str) {
        /*if let Some(buf) = self.view.get_buffer() {
            buf.set_text("");
        }
        self.sql_toggle.set_label(Some(fname));
        self.view.set_sensitive(false);
        self.sql_toggle.set_active(true);
        if let Ok(mut fl) = self.file_loaded.try_borrow_mut() {
            *fl = true;
        } else {
            println!("Could not retrieve mutable reference to file status");
        }*/
    }

    pub fn set_view_mode(&self) {
        /*if let Some(buf) = self.view.get_buffer() {
            buf.set_text("");
        }
        self.sql_toggle.set_label(None);
        self.sql_toggle.set_active(false);
        self.view.set_sensitive(true);
        if let Ok(mut fl) = self.file_loaded.try_borrow_mut() {
            *fl = false;
        } else {
            println!("Could not retrieve mutable reference to file status");
        }*/
    }

    pub fn connect_sql_load(&self, nb : TableNotebook, table_env : Rc<RefCell<TableEnvironment>>) {
        //let view =  self.view.clone();
        //let nb = nb.clone();
        //let sql_toggle = self.sql_toggle.clone();
        /*let sql_popover = self.clone();
        self.sql_load_dialog.connect_response(move |dialog, resp|{
            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            let p = path.as_path();
                            let mut sql_content = String::new();
                            if let Ok(mut f) = File::open(path.clone()) {
                                f.read_to_string(&mut sql_content);
                                t_env.prepare_query(sql_content);
                            } else {
                                println!("Unable to access informed path");
                            }
                            sql_popover.set_file_mode(p.to_str().unwrap_or(""));
                        } else {
                            sql_popover.set_view_mode();
                            t_env.clear_queries();
                        }
                    },
                    _ => {
                        sql_popover.set_view_mode();
                        t_env.clear_queries();
                    }
                }
            } else {
                println!("Unable to retrieve mutable reference to table environment");
            }
        });*/
    }

    // Action to be executed if there is text on the buffer and the user either pressed
    // the refresh button or pressed CTRL+Enter
    pub fn connect_send_query<F>(&self, f : F)
        where
            F : Fn() -> Result<(), String> + 'static,
            F : Clone
    {
        {
            let view = self.view.clone();
            let file_loaded = self.file_loaded.clone();
            let query_sent = self.query_sent.clone();
            let table_env = self.t_env.clone();
            let f = f.clone();
            self.refresh_btn.connect_clicked(move |btn|{
                match table_env.try_borrow_mut() {
                    Ok(mut env) => {
                        let update_res = Self::update_queries(
                            file_loaded.clone(),
                            query_sent.clone(),
                            &mut env,
                            &view.clone(),
                        );
                        if let Ok(_) = update_res {
                            if let Err(e) = f() {
                                println!("{}", e);
                            }
                        }
                    },
                    _ => { println!("Error recovering references"); }
                }
            });
        }
        self.connect_source_key_press(f);
    }

    /*pub fn connect_refresh(&self, /*table_env : Rc<RefCell<TableEnvironment>>, tables_nb : TableNotebook*/ ) {
        let view = self.view.clone();
        let file_loaded = self.file_loaded.clone();
        let query_sent = self.query_sent.clone();
        let table_env = self.t_env.clone();
        self.refresh_btn.connect_clicked(move|btn|{
            match table_env.try_borrow_mut() {
                Ok(mut env) => {
                    //println!("before update: {}", query_sent.borrow());
                    Self::update_queries(
                        file_loaded.clone(),
                        query_sent.clone(),
                        &mut env,
                        &view.clone(),
                        //&tables_nb.clone()
                    );
                    //println!("after update: {}", query_sent.borrow());
                },
                _ => { println!("Error recovering references"); }
            }
        });
    }*/

    fn connect_source_key_press<F>(&self, f : F)
        where
            F : Fn() -> Result<(), String> + 'static,
            F : Clone
    {
        //let sql_popover = self.clone();
        let file_loaded = self.file_loaded.clone();
        let query_sent = self.query_sent.clone();
        let table_env = self.t_env.clone();
        self.view.connect_key_press_event(move |view, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK && ev_key.get_keyval() == key::Return {
                match table_env.try_borrow_mut() {
                    Ok(mut env) => {
                        let update_res = Self::update_queries(
                            file_loaded.clone(),
                            query_sent.clone(),
                            &mut env,
                            &view.clone(),
                        );
                        if let Ok(_) = update_res {
                            if let Err(e) = f() {
                                println!("{}", e);
                            }
                        }
                    },
                    _ => { println!("Error recovering references"); }
                }
                glib::signal::Inhibit(true)
            } else {
                glib::signal::Inhibit(false)
            }
        });
    }
}
