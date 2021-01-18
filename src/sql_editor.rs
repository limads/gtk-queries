use glib;
use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use gdk::{self, keys};
use crate::tables::{environment::TableEnvironment, environment::EnvironmentUpdate};
use sourceview::{self, *};
use gtk::prelude::*;
use crate::status_stack::StatusStack;
use crate::status_stack::*;
use sourceview::View;
use super::file_list::FileList;
use std::io::Write;
use std::path::Path;
use crate::schema_tree::SchemaTree;
use crate::utils;
use super::table_notebook::{TableNotebook, TableBar};
use crate::plots::plot_workspace::PlotWorkspace;
// use crate::table_popover::TablePopover;
use crate::header_toggle::HeaderToggle;
use crate::table_notebook::TableSource;

pub enum ExecStatus {
    File(String, usize),
    View(String, usize),
    None
}

#[derive(Clone)]
pub struct SqlEditor {
    pub file_list : FileList,

    /// The view is dinamically changed whenever a new file is selected. All views are owned
    /// by the content_stack->ScrolledWindow at which they are inserted. Since the view can
    /// be mutated, we wrap it in a RefCell since this change will happen at an Fn closure when
    /// a new file is selected at FileList.
    pub view : Rc<RefCell<View>>,
    pub sql_load_dialog : FileChooserDialog,
    pub sql_save_dialog : FileChooserDialog,

    pub refresh_btn : Button,
    clear_btn : ToolButton,
    update_btn : ToggleButton,

    pub query_sent : Rc<RefCell<bool>>,
    pub sql_stack : Stack,
    pub status_stack : StatusStack,
    t_env : Rc<RefCell<TableEnvironment>>,
    // sql_new_btn : Button,
    // sql_load_btn : Button,
    query_file_label : Label,
    table_toggle : ToggleButton,

    // Keeps status if clock was started at first position,
    // the update interval at second position (constant) and
    // time that ran since the last update (updated at each glib::timeout)
    // at third position.
    update_clock : Rc<RefCell<(bool, usize, usize)>>
}

impl SqlEditor {

    fn save_file(path : &Path, content : String) -> bool {
        if let Ok(mut f) = File::create(path) {
            if f.write_all(content.as_bytes()).is_ok() {
                println!("Content written to file");
                true
            } else {
                false
            }
        } else {
            println!("Unable to write into file");
            false
        }
    }

    pub fn save_current(&self) {
        if let Some(path) = self.file_list.current_selected_path() {
            if Self::save_file(&path, self.get_text()) {
                self.file_list.mark_current_saved();
                println!("Content written into file");
            } else {
                println!("Unable to save file");
            }
        } else {
            self.sql_save_dialog.run();
            self.sql_save_dialog.hide();
        }
    }

    fn connect_sql_save(editor : &SqlEditor) {
        let sql_save_dialog = editor.sql_save_dialog.clone();
        let editor = editor.clone();
        sql_save_dialog.connect_response(move |dialog, resp|{
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        if Self::save_file(path.as_path(), editor.get_text()) {
                            editor.file_list.set_current_selected_path(path.as_path());
                            editor.file_list.mark_current_saved();
                            println!("Content written to file");
                        } else {
                            println!("Unable to write to file");
                        }
                    } else {
                        println!("Unable to retrieve path");
                    }
                },
                _ => { },
            }
            dialog.hide();
        });
    }

    pub fn add_fresh_editor(&self, content_stack : Stack, query_toggle : ToggleButton) {
        if self.file_list.get_selected().is_none() {
            self.file_list.add_file_row(
                "Untitled 1",
                content_stack.clone(),
                self.clone(),
            );
            content_stack.set_visible_child_name("queries_0");
            self.file_list.select_last();
        } else {
            self.file_list.add_fresh_source(
                content_stack.clone(),
                self.clone(),
                query_toggle.clone()
            );
        }
    }

    pub fn new_source(content : &str, refresh_btn : &Button, file_list : &FileList) -> ScrolledWindow {
        let no_adj : Option<&Adjustment> = None;
        let sw = ScrolledWindow::new(no_adj, no_adj);
        let view = View::new();
        Self::configure_view(&view, refresh_btn, file_list.clone());
        view.get_buffer().map(|buf| buf.set_text(&content) );
        sw.add(&view);
        sw
    }

    pub fn update_editor(&self, content_stack : Stack, new_name : &str) {
        let new_view = content_stack.get_child_by_name(&new_name)
            .and_then(|child| child.downcast::<ScrolledWindow>().ok() )
            .and_then(|sw| {
                let child = sw.get_child().unwrap();
                child.downcast::<View>().ok()
            });
        if let Some(view) = new_view {
            *(self.view.borrow_mut()) = view;
            // println!("Updating SQL editor to {:?}", view);
        } else {
            println!("Could not retrieve new view");
        }
    }

    pub fn get_text(&self) -> String {
        if let Some(buffer) = self.view.borrow().get_buffer() {
            let txt = buffer.get_text(
                &buffer.get_start_iter(),
                &buffer.get_end_iter(),
                true
            ).unwrap();
            txt.to_string()
        } else {
            panic!("Unable to retrieve text buffer");
        }
    }

    pub fn set_text(&self, txt : &str) {
        if let Some(buffer) = self.view.borrow().get_buffer() {
            buffer.set_text(txt);
        } else {
            println!("Unable to retrieve text buffer");
        }
    }

    pub fn set_active(&self, state : bool) {
        // self.sql_new_btn.set_sensitive(state);
        // self.sql_load_btn.set_sensitive(state);
        self.clear_btn.set_sensitive(state);
        self.update_btn.set_sensitive(state);
        self.refresh_btn.set_sensitive(state);

        if state == false {
            self.sql_load_dialog.unselect_all();
            if let Some(buffer) = self.view.borrow().get_buffer() {
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
        //file_loaded : Rc<RefCell<bool>>,
        query_sent : Rc<RefCell<bool>>,
        tbl_env : &mut TableEnvironment,
        view : &sourceview::View,
        // status_stack : StatusStack
        // nb : &TableNotebook
    ) -> Result<(), String> {
        /*if let Ok(loaded) = file_loaded.try_borrow() {
            if *loaded {
                tbl_env.send_current_query(true)?;
                view.set_sensitive(false);
                //nb.nb.set_sensitive(false);
            } else {*/
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
                    println!("Updating query: {}", txt);
                    tbl_env.prepare_and_send_query(txt, true)?;
                    view.set_sensitive(false);
                    // file_list.set_sensitive(false);
                    // nb.nb.set_sensitive(false);
                } else {
                    println!("No text available to send");
                }
            } else {
                return Err(format!("Could not retrieve text buffer"));
            }
        /*} else {
            return Err(format!("Could not retrieve reference to file status"));
        }*/
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

    // TODO if the results arrived before the first iteration of this  function
    // the procedure will fail. Verify if there isn't any updates queued before issuing
    // command.

    pub fn connect_result_arrived<F>(
        &self,
        schema_tree : SchemaTree,
        mut f : F
    )
    where
        F : FnMut(&TableEnvironment, &EnvironmentUpdate) -> Result<(), String> + 'static
    {
        let tbl_env_c = self.t_env.clone();
        let status_stack = self.status_stack.clone();
        let view_c = self.view.clone();
        let sql_editor = self.clone();
        let table_toggle = self.table_toggle.clone();
        let file_list = self.file_list.clone();
        let n_tries = Rc::new(RefCell::new(0));
        glib::timeout_add_local(16, move || {
            let mut req_tree_update = false;
            if let Ok(mut sent) = sql_editor.query_sent.try_borrow_mut() {
                if *sent {
                    // println!("Sent");
                    // println!("{}", sql_popover.query_sent.borrow());
                    if let Ok(mut t_env) = tbl_env_c.try_borrow_mut() {
                        let last_cmds = t_env.last_commands();
                        println!("Last commands: {:?}", last_cmds);
                        let updated = if let Some(last_cmd) = last_cmds.last() {
                            // println!("Query updated");
                            // println!("Last command: {}", last_cmd);
                            if &last_cmd[..] == "select" {
                                match t_env.maybe_update_from_query_results() {
                                    Some(ans) => {
                                        if t_env.any_modification_result() {
                                            req_tree_update = true;
                                        }
                                        match ans {
                                            Ok(ref update) => {
                                                if let Err(e) = f(&t_env, update) {
                                                    println!("{}", e);
                                                    status_stack.update(Status::SqlErr(e));
                                                    table_toggle.set_active(true);
                                                } else {
                                                    status_stack.update(Status::Ok);
                                                }
                                            },
                                            Err(e) => {
                                                println!("{}", e);
                                                status_stack.update(Status::SqlErr(e));
                                                table_toggle.set_active(true);
                                            }
                                        }
                                        true
                                    },
                                    None => false
                                }
                            } else {
                                match t_env.maybe_update_from_statement() {
                                    Some(ans) => {
                                        if t_env.any_modification_result() {
                                            req_tree_update = true;
                                        }
                                        match ans {
                                            Ok(msg) => {
                                                /*let is_create = msg.starts_with("Create");
                                                let is_alter = msg.starts_with("Alter");
                                                let is_drop = msg.starts_with("Drop");
                                                req_tree_update = is_create || is_alter || is_drop;*/
                                                status_stack.update(Status::StatementExecuted(msg));
                                                table_toggle.set_active(true);
                                            },
                                            Err(e) => {
                                                println!("{}", e);
                                                status_stack.update(Status::SqlErr(e));
                                                table_toggle.set_active(true);
                                            }
                                        }
                                        true
                                    },
                                    None => {
                                        println!("No result from last statement");
                                        false
                                    }
                                }
                            }
                        } else {
                            println!("Unable to retrieve last command");
                            status_stack.update(Status::SqlErr(format!("Unable to retrieve last command")));
                            table_toggle.set_active(true);
                            false
                        };
                        if updated {
                            view_c.borrow().set_sensitive(true);
                            file_list.set_sensitive(true);
                            view_c.borrow().grab_focus();
                            *sent = false;
                            // println!("Sent set to false");
                            *(n_tries.borrow_mut()) = 0; 
                        } else {
                            // println!("Not updated yet");
                            let mut tries = n_tries.borrow_mut();
                            *tries += 1;
                            if *tries == 312 {
                                view_c.borrow().set_sensitive(true);
                                file_list.set_sensitive(true);
                                view_c.borrow().grab_focus();
                                *sent = false;
                                println!("Sent set to false");
                                *tries = 0; 
                                status_stack.update(Status::SqlErr(format!("5 seconds timeout reached")));
                                table_toggle.set_active(true);
                            }
                        }
                    } else {
                        println!("Unable to retrieve mutable reference to table environment");
                        return glib::source::Continue(true);
                    };
                }
            } else {
                println!("Unable to retrieve reference to query sent status");
            }

            if req_tree_update {
                schema_tree.repopulate(tbl_env_c.clone());
            }

            glib::source::Continue(true)
        });
    }

    // TODO Query the folders /usr/share/gtksourceview-4/styles or
    // /usr/local/share/gtksourceview-4/styles for styles or
    // /usr/share/gtksourceview-3.0/styles
    fn configure_view(view : &View, refresh_btn : &Button, file_list : FileList) {
        let buffer = view.get_buffer().unwrap()
            .downcast::<sourceview::Buffer>().unwrap();
        let manager = StyleSchemeManager::new();
        println!("available schemes: {:?}", manager.get_scheme_ids());
        let scheme = manager.get_scheme("queries").unwrap();
        buffer.set_style_scheme(Some(&scheme));
        buffer.set_highlight_syntax(true);
        // view.reset_style();
        println!("{:?}", buffer.get_style_scheme().unwrap().get_id());
        let provider = CssProvider::new();
        provider.load_from_data(b"textview { font-family: \"Ubuntu Mono\"; font-size: 13pt; }").unwrap();
        let ctx = view.get_style_context();
        ctx.add_provider(&provider, 800);

        let lang_manager = LanguageManager::get_default().unwrap();
        let lang = lang_manager.get_language("sql").unwrap();
        buffer.set_language(Some(&lang));
        Self::connect_source_key_press(&view, &refresh_btn);
        let buffer = view.get_buffer().unwrap();
        buffer.connect_changed(move |_buf| {
            file_list.mark_current_unsaved();
        });

        view.set_tab_width(4);
        view.set_indent_width(4);
        view.set_auto_indent(true);
        view.set_insert_spaces_instead_of_tabs(true);
        view.set_right_margin(80);
        view.set_highlight_current_line(true);
        view.set_indent_on_tab(true);
        view.set_show_line_marks(true);
        // view.set_background_pattern(BackgroundPatternType::Grid);
        view.set_right_margin_position(80);
        view.set_show_right_margin(true);
        view.set_show_line_numbers(false);
    }

    pub fn build(
        builder : Builder,
        header_toggle : HeaderToggle,
        status_stack : StatusStack,
        content_stack : Stack,
        t_env : Rc<RefCell<TableEnvironment>>,
        tables_nb : TableNotebook,
        file_list : &FileList,
        workspace : PlotWorkspace,
        table_bar : TableBar
    ) -> Self {
        let view : View =
            builder.get_object("query_source").unwrap();

        let sql_stack : Stack = builder.get_object("sql_stack").unwrap();
        sql_stack.set_visible_child_name("empty");

        // let sql_new_btn : Button = builder.get_object("sql_new_btn").unwrap();
        //let sql_load_btn : Button = builder.get_object("sql_load_btn").unwrap();
        let query_file_label : Label = builder.get_object("query_file_label").unwrap();
        /*{
            let sql_stack = sql_stack.clone();
            sql_new_btn.connect_clicked(move |_btn| {
                sql_stack.set_visible_child_name("source");
            });
        }*/
        let sql_load_dialog : FileChooserDialog =
            builder.get_object("sql_load_dialog").unwrap();
        /*{
            let sql_load_dialog = sql_load_dialog.clone();
            sql_load_btn.connect_clicked(move |_btn| {
                sql_load_dialog.run();
                sql_load_dialog.hide();
            });
        }*/
        //let extra_toolbar : Toolbar = builder.get_object("extra_toolbar").unwrap();
        let sql_toolbar : Toolbar = builder.get_object("sql_toolbar").unwrap();
        let img_clear = Image::from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let clear_btn = ToolButton::new(Some(&img_clear), None);
        //let img_refresh = Image::new_from_icon_name(Some("view-refresh"), IconSize::SmallToolbar);
        let refresh_btn : Button = builder.get_object("refresh_btn").unwrap();
        let update_btn : ToggleButton = builder.get_object("update_btn").unwrap();
        //let img_clock = Image::new_from_icon_name(Some("clock-app-symbolic"), IconSize::SmallToolbar);
        //update_btn.set_icon_widget(Some(&img_clock));
        clear_btn.set_sensitive(false);
        update_btn.set_sensitive(false);
        refresh_btn.set_sensitive(false);
        //sql_toolbar.insert(&refresh_btn, 2);
        sql_toolbar.insert(&clear_btn, 0);
        //sql_toolbar.insert(&update_btn, 1);
        sql_toolbar.show_all();

        Self::configure_view(&view, &refresh_btn, file_list.clone());

        let update_clock = Rc::new(RefCell::new((false, 0, 0)));
        {
            let update_clock = update_clock.clone();
            // let refresh_btn = refresh_btn.clone();
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
            glib::timeout_add_local(500, move || {
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

        // let img_append = Image::new_from_file(utils::glade_path("../icons/append.svg").unwrap());
        // let append_btn = ToggleToolButton::new();
        // append_btn.set_icon_widget(Some(&img_append));

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
        let sql_save_dialog : FileChooserDialog =
            builder.get_object("sql_save_dialog").unwrap();

        let sql_editor = Self {
            view : Rc::new(RefCell::new(view)),
            //sql_load_dialog,
            refresh_btn,
            //popover,
            //query_toggle,
            sql_load_dialog,
            sql_save_dialog,
            //sql_box,
            //extra_toolbar,
            // sql_toggle,
            // file_loaded,
            query_sent : Rc::new(RefCell::new(false)),
            sql_stack,
            status_stack,
            t_env,
            update_clock,
            clear_btn,
            update_btn,
            //sql_new_btn,
            //sql_load_btn,
            query_file_label,
            table_toggle : header_toggle.table_toggle.clone(),
            file_list : file_list.clone(),
        };

        Self::connect_sql_load(
            sql_editor.sql_load_dialog.clone(),
            sql_editor.t_env.clone(),
            tables_nb.clone(),
            // sql_editor.query_file_label.clone(),
            &file_list,
            content_stack,
            header_toggle.query_toggle.clone(),
            sql_editor.refresh_btn.clone(),
            sql_editor.clone(),
            workspace,
            table_bar,
            sql_editor.status_stack.clone()
        );

        Self::connect_sql_save(&sql_editor);

        sql_editor
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

    pub fn clip_name(path : &Path) -> Option<String> {
        if let Some(name) = path.file_name().and_then(|n| n.to_str() ) {
            if let Some(parent) = path.parent().and_then(|p| p.to_str() ) {
                Some(format!("{}/{}", parent, name))
            } else {
                Some(name.to_string())
            }
        } else {
            println!("Invalid path name");
            None
        }
    }

    fn connect_sql_load(
        sql_load_dialog : FileChooserDialog,
        table_env : Rc<RefCell<TableEnvironment>>,
        tables_nb : TableNotebook,
        // query_file_label : Label,
        file_list : &FileList,
        content_stack : Stack,
        query_toggle : ToggleButton,
        refresh_btn : Button,
        sql_editor : SqlEditor,
        workspace : PlotWorkspace,
        table_bar : TableBar,
        status_stack : StatusStack
    ) {
        let file_list = file_list.clone();
        sql_load_dialog.connect_response(move |dialog, resp|{
            //if let Ok(mut t_env) = table_env.try_borrow_mut() {
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        match path.extension().and_then(|ext| ext.to_str() ) {
                            Some("sql") => {
                                if let Some(name) = Self::clip_name(&path) {
                                    println!("Adding disk source file: {:?}", name);
                                    file_list.add_disk_file(
                                        path.as_path(),
                                        &name[..],
                                        content_stack.clone(),
                                        query_toggle.clone(),
                                        refresh_btn.clone(),
                                        sql_editor.clone()
                                    );
                                } else {
                                    println!("Could not retrieve name");
                                }
                            },
                            Some("csv") => {
                                let mut txt = String::new();
                                if let Ok(mut f) = File::open(&path) {
                                    if let Err(e) = f.read_to_string(&mut txt) {
                                        println!("{}", e);
                                    }
                                    if let Some(name) = Self::clip_name(&path) {
                                        let add_res = utils::add_external_table(
                                            &table_env,
                                            &tables_nb,
                                            TableSource::File(name),
                                            txt,
                                            &workspace,
                                            &table_bar,
                                            &status_stack
                                        );
                                        if let Err(e) = add_res {
                                            println!("{}", e);
                                        }
                                    } else {
                                        println!("Unable to retrieve file name");
                                    }
                                } else {
                                    println!("Unable to open external file");
                                }
                            },
                            _ => {
                                println!("Extension should be SQL or CSV");
                            }
                        }
                    } else {
                        println!("Could not get path");
                        return;
                    };
                },
                _ => {
                    // sql_popover.set_view_mode();
                    // t_env.clear_queries();
                    // query_file_label.set_text("Empty query sequence");
                }
            }
            dialog.hide();
            /*} else {
                println!("Unable to retrieve mutable reference to table environment");
                query_file_label.set_text("Empty query sequence");
            }*/
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
            // println!("Now sending sql query: {:?}", self.get_text());
            let view = self.view.clone();
            let query_sent = self.query_sent.clone();
            let table_env = self.t_env.clone();
            let update_clock = self.update_clock.clone();
            let f = f.clone();
            self.refresh_btn.connect_clicked(move |_btn|{
                match table_env.try_borrow_mut() {
                    Ok(mut env) => {
                        let update_res = Self::update_queries(
                            //file_loaded.clone(),
                            query_sent.clone(),
                            &mut env,
                            &view.borrow().clone(),
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
        // Self::connect_source_key_press(&*self.view.borrow(), &self.refresh_btn);
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

    fn connect_source_key_press /*<F>*/ (view : &View, refresh_btn : &Button/*f : F*/)
        //where
        //    F : Fn(Result<(), String>) -> Result<(), String> + 'static,
        //    F : Clone
    {
        // let sql_popover = self.clone();
        // let file_loaded = self.file_loaded.clone();
        // let query_sent = self.query_sent.clone();
        // let table_env = self.t_env.clone();
        let refresh_btn = refresh_btn.clone();
        // TODO verify that view is realized before accepting key press
        view.connect_key_press_event(move |_view, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK && ev_key.get_keyval() == keys::constants::Return {
                println!("Return clicked");
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
