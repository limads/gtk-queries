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
use crate::tables::{self, source::EnvironmentSource, environment::TableEnvironment, sql::SqlListener};
use sourceview::*;
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use crate::{utils, table_widget::TableWidget, table_notebook::TableNotebook, status_stack::StatusStack };
use crate::tables::table::Table;
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
    pub sql_load_dialog : FileChooserDialog,
    pub refresh_btn : ToolButton,
    clear_btn : ToolButton,
    update_btn : ToggleToolButton,
    //pub popover : Popover,
    //pub query_toggle : ToggleButton,
    //pub sql_toggle : ToggleToolButton,
    pub file_loaded : Rc<RefCell<bool>>,
    pub query_sent : Rc<RefCell<bool>>,
    //pub sql_box : Box,
    //pub extra_toolbar : Toolbar,
    pub sql_stack : Stack,
    pub status_stack : StatusStack,
    t_env : Rc<RefCell<TableEnvironment>>,
    sql_new_btn : Button,
    sql_load_btn : Button,
    query_file_label : Label,

    // Keeps status if clock was started at first position,
    // the update interval at second position (constant) and
    // time that ran since the last update (updated at each glib::timeout)
    // at third position.
    update_clock : Rc<RefCell<(bool, usize, usize)>>
}

impl SqlPopover {

    pub fn set_active(&self, state : bool) {
        // self.sql_new_btn.set_sensitive(state);
        // self.sql_load_btn.set_sensitive(state);
        self.clear_btn.set_sensitive(state);
        self.update_btn.set_sensitive(state);
        self.refresh_btn.set_sensitive(state);
        if state == false {
            self.sql_load_dialog.unselect_all();
            if let Some(buffer) = self.view.get_buffer() {
                buffer.set_text("");
            } else {
                println!("Unable to retrieve text buffer");
            }
            self.update_btn.set_active(false);
            if let Ok(mut clock) = self.update_clock.try_borrow_mut() {
                clock.0 = false;
                clock.1 = 0;
                clock.2 = 0;
            } else {
                println!("Unable to retrieve mutable reference to update clock");
            }
        }
    }

    /// Update the query. If there was a SQL parsing error,
    /// return it. If there was no error, set the SQL sourceview
    /// to insensitive (until no result arrived) and return Ok(()).
    pub fn update_queries(
        file_loaded : Rc<RefCell<bool>>,
        query_sent : Rc<RefCell<bool>>,
        tbl_env : &mut TableEnvironment,
        view : &sourceview::View,
        // status_stack : StatusStack
        // nb : &TableNotebook
    ) -> Result<(), String> {
        if let Ok(loaded) = file_loaded.try_borrow() {
            if *loaded {
                tbl_env.send_current_query()?;
                view.set_sensitive(false);
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
                        // println!("{}", txt);
                        tbl_env.prepare_and_send_query(txt)?;
                        view.set_sensitive(false);
                        // nb.nb.set_sensitive(false);
                    } else {
                        println!("No text available to send");
                    }
                } else {
                    return Err(format!("Could not retrieve text buffer"));
                }
            }
        } else {
            return Err(format!("Could not retrieve reference to file status"));
        }
        if let Ok(mut sent) = query_sent.try_borrow_mut() {
            *sent = true;
            //println!("Query sent");
        } else {
            return Err(format!("Unable to acquire lock over query sent status"))
        }
        Ok(())
        //println!("at update: {}", query_sent.borrow());
    }

    /*pub fn add_extra_toolbar(&self, tool_btn : ToggleToolButton, bx : gtk::Box) {
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
    }*/

    // TODO change stack to table any time a new query result arrives.
    // But toggle should stay off (full table view).

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

    fn build_load_btn(&self) {

    }

    pub fn new(builder : Builder, /*query_toggle : ToggleButton,*/ status_stack : StatusStack, t_env : Rc<RefCell<TableEnvironment>>) -> Self {
        //let query_popover_path = utils::glade_path("query-popover-3.glade").expect("Failed to load glade file");
        //let builder = Builder::new_from_file(query_popover_path);
        //let popover : Popover =
        //    builder.get_object("query_popover").unwrap();
        //let sql_box : Box = builder.get_object("sql_box").unwrap();
        let view : View =
            builder.get_object("query_source").unwrap();
        //view.realize();
        let buffer = view.get_buffer().unwrap()
            .downcast::<sourceview::Buffer>().unwrap();
        let lang_manager = LanguageManager::get_default().unwrap();
        let lang = lang_manager.get_language("sql").unwrap();
        buffer.set_language(Some(&lang));
        let sql_stack : Stack = builder.get_object("sql_stack").unwrap();
        sql_stack.set_visible_child_name("empty");
        let sql_new_btn : Button = builder.get_object("sql_new_btn").unwrap();
        let sql_load_btn : Button = builder.get_object("sql_load_btn").unwrap();
        let query_file_label : Label = builder.get_object("query_file_label").unwrap();
        {
            let sql_stack = sql_stack.clone();
            sql_new_btn.connect_clicked(move |btn| {
                sql_stack.set_visible_child_name("source");
            });
        }
        let sql_load_dialog : FileChooserDialog =
            builder.get_object("sql_load_dialog").unwrap();
        {
            let sql_load_dialog = sql_load_dialog.clone();
            sql_load_btn.connect_clicked(move |btn| {
                sql_load_dialog.run();
                sql_load_dialog.hide();
            });
        }
        //let extra_toolbar : Toolbar = builder.get_object("extra_toolbar").unwrap();
        let sql_toolbar : Toolbar = builder.get_object("sql_toolbar").unwrap();
        let img_clear = Image::new_from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let clear_btn = ToolButton::new(Some(&img_clear), None);
        let img_refresh = Image::new_from_icon_name(Some("view-refresh"), IconSize::SmallToolbar);
        let refresh_btn : ToolButton = ToolButton::new(Some(&img_refresh), None);
        let update_btn = ToggleToolButton::new();
        let img_clock = Image::new_from_icon_name(Some("clock-app-symbolic"), IconSize::SmallToolbar);
        update_btn.set_icon_widget(Some(&img_clock));
        clear_btn.set_sensitive(false);
        update_btn.set_sensitive(false);
        refresh_btn.set_sensitive(false);
        sql_toolbar.insert(&refresh_btn, 2);
        sql_toolbar.insert(&clear_btn, 0);
        sql_toolbar.insert(&update_btn, 1);
        sql_toolbar.show_all();

        let update_clock = Rc::new(RefCell::new((false, 0, 0)));
        {
            let update_clock = update_clock.clone();
            let refresh_btn = refresh_btn.clone();
            update_btn.connect_toggled(move |btn| {
                if let Ok(mut update) = update_clock.try_borrow_mut() {
                    update.0 = false;
                    if btn.get_active() {
                        update.1 = 2000;
                    } else {
                        update.1 = 0;
                        //if !refresh_btn.is_sensitive() {
                            //refresh_btn.set_sensitive(true);
                        //}
                    }
                } else {
                    println!("Failed to retrieve mutable reference to refresh state");
                }
            });
        }

        {
            let update_clock = update_clock.clone();
            let refresh_btn = refresh_btn.clone();
            gtk::timeout_add(500, move || {
               if let Ok(mut update) = update_clock.try_borrow_mut() {
                    match *update {
                        (false, _, _) => {

                        },
                        (true, 0, _) => {

                        },
                        (true, interval, passed) => {
                            if passed >= interval {
                                update.2 = 0;
                                if refresh_btn.is_sensitive() {
                                    println!("Updating query at {:?}", update);
                                    refresh_btn.emit_clicked();
                                }
                            } else {
                                update.2 += 500;
                                //println!("Waiting next update: {:?}", update);
                            }
                        }
                    }
                } else {
                    println!("Unable to retreive mutable reference to update clock");
                }
                glib::source::Continue(true)
            });
        }

        //let img_append = Image::new_from_file(utils::glade_path("../icons/append.svg").unwrap());
        //let append_btn = ToggleToolButton::new();
        //append_btn.set_icon_widget(Some(&img_append));

        {
            let t_env = t_env.clone();
            let sql_stack = sql_stack.clone();
            let query_file_label = query_file_label.clone();
            clear_btn.connect_clicked(move |_btn| {
                if let Ok(mut t_env) = t_env.try_borrow_mut() {
                    t_env.prepare_query(String::new());
                } else {
                    println!("Error fetching mutable reference to table environment");
                }
                sql_stack.set_visible_child_name("empty");
                query_file_label.set_text("Empty query sequence");
            });
        }
        //sql_toolbar.insert(&append_btn, 2);
        // update_toolbar.insert(&item_b, 1);
        // extra_toolbar.show_all();
        //update_btn.connect_toggled(move|btn|{
            /*let curr_label = btn.get_label().unwrap();
            let new_label = match curr_label.as_str() {
                "Off" => "0.5 s",
                "0.5 s" => "1 s",
                "1 s" => "5 s",
                "5 s" => "Off",
                _ => "Off"
            };
            btn.set_label(Some(new_label));*/
        //});

        /*{
            let view = view.clone();

            {
                let sql_load_dialog = sql_load_dialog.clone();

            }
        }*/

        //let exec_toolbar : Toolbar = builder.get_object("exec_toolbar").unwrap();


        /*{
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

        popover.set_relative_to(Some(&query_toggle));*/

        let file_loaded = Rc::new(RefCell::new(false));
        Self::connect_sql_load(sql_load_dialog.clone(), t_env.clone(), file_loaded.clone(), query_file_label.clone());

        Self {
            view,
            //sql_load_dialog,
            refresh_btn,
            //popover,
            //query_toggle,
            sql_load_dialog,
            //sql_box,
            //extra_toolbar,
            // sql_toggle,
            file_loaded,
            query_sent : Rc::new(RefCell::new(false)),
            sql_stack,
            status_stack,
            t_env,
            update_clock,
            clear_btn,
            update_btn,
            sql_new_btn,
            sql_load_btn,
            query_file_label
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

    pub fn connect_sql_load(
        sql_load_dialog : FileChooserDialog,
        table_env : Rc<RefCell<TableEnvironment>>,
        file_loaded : Rc<RefCell<bool>>,
        query_file_label : Label
    ) {
        //let sql_popover = self.clone();
        sql_load_dialog.connect_response(move |dialog, resp|{
            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                if let Ok(mut loaded) = file_loaded.try_borrow_mut() {
                    match resp {
                        ResponseType::Other(1) => {
                            if let Some(path) = dialog.get_filename() {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str() ) {
                                    query_file_label.set_text(name);
                                } else {
                                    query_file_label.set_text("(Unknown path)");
                                }
                                let mut sql_content = String::new();
                                if let Ok(mut f) = File::open(path.clone()) {
                                    if let Err(e) = f.read_to_string(&mut sql_content) {
                                        println!("{}", e);
                                    }
                                    t_env.prepare_query(sql_content);
                                    *loaded = true;
                                } else {
                                    println!("Unable to access informed path");
                                    *loaded = false;
                                    query_file_label.set_text("Empty query sequence");
                                }
                            } else {
                                t_env.clear_queries();
                                *loaded = false;
                                query_file_label.set_text("Empty query sequence");
                            }
                        },
                        _ => {
                            //sql_popover.set_view_mode();
                            t_env.clear_queries();
                            *loaded = false;
                            query_file_label.set_text("Empty query sequence");
                        }
                    }
                } else {
                    println!("Unable to acquire lock over file loaded status");
                    query_file_label.set_text("Empty query sequence");
                }
            } else {
                println!("Unable to retrieve mutable reference to table environment");
                query_file_label.set_text("Empty query sequence");
            }
        });
    }

    /// Action to be executed if there is text on the buffer or a loaded SQL file and the user
    /// either pressed the refresh button or pressed CTRL+Enter.
    /// The callback informed by the user receive a Result<(), String> which carries the
    /// SQL parsing status with an error message if the SQL could not be parsed.
    pub fn connect_send_query<F>(&self, f : F)
        where
            F : Fn(Result<(), String>) -> Result<(), String> + 'static,
            F : Clone
    {
        {
            let view = self.view.clone();
            let file_loaded = self.file_loaded.clone();
            let query_sent = self.query_sent.clone();
            let table_env = self.t_env.clone();
            let update_clock = self.update_clock.clone();
            let f = f.clone();
            self.refresh_btn.connect_clicked(move |_btn|{
                match table_env.try_borrow_mut() {
                    Ok(mut env) => {
                        let update_res = Self::update_queries(
                            file_loaded.clone(),
                            query_sent.clone(),
                            &mut env,
                            &view.clone()
                        );
                        if let Err(e) = f(update_res) {
                            println!("{}", e);
                        }
                    },
                    _ => { println!("Error recovering mutable reference to table environment"); }
                }
                if let Ok(mut update) = update_clock.try_borrow_mut() {
                    if update.1 > 0 {
                        update.0 = true;
                        update.2 = 0;
                        //btn.set_sensitive(false);
                    }
                } else {
                    println!("Unabe to recover mutable reference to update clock");
                }
            });
        }
        self.connect_source_key_press( /*f*/ );
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

    fn connect_source_key_press /*<F>*/ (&self, /*f : F*/)
        //where
        //    F : Fn(Result<(), String>) -> Result<(), String> + 'static,
        //    F : Clone
    {
        // let sql_popover = self.clone();
        // let file_loaded = self.file_loaded.clone();
        // let query_sent = self.query_sent.clone();
        // let table_env = self.t_env.clone();
        let refresh_btn = self.refresh_btn.clone();
        // TODO verify that view is realized before accepting key press
        self.view.connect_key_press_event(move |view, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK && ev_key.get_keyval() == key::Return {
                if refresh_btn.is_sensitive() {
                    refresh_btn.emit_clicked();
                    /*match table_env.try_borrow_mut() {
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
                    }*/
                }
                glib::signal::Inhibit(true)
            } else {
                glib::signal::Inhibit(false)
            }
        });
    }
}
