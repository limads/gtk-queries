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
use gtk_queries::conn_popover::*;
use sourceview::*;
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use gtk_queries::{utils, table_widget::TableWidget, table_notebook::TableNotebook };
use nlearn::table::Table;
use gtk_queries::status_stack::*;
use gtk_queries::sql_popover::*;
use gtk_queries::functions::function_search::*;
use gtk_queries::functions::num_function::*;
use gdk::prelude::*;
use gtkplotview::plot_view::PlotView;
use gtk_plots::save_widgets;
use gtk_queries::plots::layout_menu::PlotSidebar;

#[derive(Clone)]
pub struct QueriesApp {
    //exec_btn : Button,
    //view : sourceview::View,
    //tables_nb : TableNotebook,
    // file_btn : FileChooserButton,
    //header : HeaderBar,
    // unsaved_dialog : Dialog,
    // new_file : Rc<RefCell<bool>>,
    // unsaved_changes : Rc<RefCell<bool>>,
    // save_dialog : Dialog,
    conn_popover : ConnPopover,
    sql_popover : SqlPopover,
    table_env : Rc<RefCell<TableEnvironment>>,
    //query_toggle : ToggleButton //,
    //ws_toggle : ToggleButton

    // old_source_content : Rc<RefCell<String>>
}

pub fn set_tables(
    table_env : &TableEnvironment,
    tables_nb : &mut TableNotebook,
    fn_search : FunctionSearch
) {
    tables_nb.clear();
    let all_tbls = table_env.all_tables_as_rows();
    if all_tbls.len() == 0 {
        tables_nb.add_page(
            "application-exit",
            None,
            Some("No queries"),
            None,
            fn_search.clone()
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
                    fn_search.clone()
                );
            } else {
                println!("No rows to display");
            }
        }
    }
}

fn ajust_sidebar_pos(btn : &ToggleButton, window : &Window, main_paned : &Paned) {
    if let Some(win) = window.get_window() {
        let w = win.get_width();
        match btn.get_active() {
            false => {
                main_paned.set_position(w);
            },
            true => {
                let adj_w = (w as f32 * 0.8) as i32;
                main_paned.set_position(adj_w);
            }
        }
    }
}

impl QueriesApp {

    fn build_plots_widgets(
        table_env : Rc<RefCell<TableEnvironment>>,
        pl_da : DrawingArea,
        sidebar_stack : Stack
    ) -> PlotSidebar {
        let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let pl_view = PlotView::new_with_draw_area(
            "assets/plot_layout/layout.xml", pl_da.clone());
        save_widgets::build_save_widgets(&builder, pl_view.clone());
        let sidebar = PlotSidebar::new(pl_view.clone(), table_env.clone());
        sidebar_stack.add_named(&sidebar.layout_stack, "layout");
        sidebar
    }

    fn build_toggles(
        builder : Builder,
        sidebar_stack : Stack,
        content_stack : Stack
    ) {
        let main_paned : Paned =  builder.get_object("main_paned").unwrap();
        let table_toggle : ToggleButton = builder.get_object("table_toggle").unwrap();
        let plot_toggle : ToggleButton = builder.get_object("plot_toggle").unwrap();
        {
            let main_paned = main_paned.clone();
            let plot_toggle = plot_toggle.clone();
            let sidebar_stack = sidebar_stack.clone();
            let content_stack = content_stack.clone();
            table_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        main_paned.set_position(0);
                        if plot_toggle.get_active() {
                            //plot_toggle.set_active(false);
                            //plot_toggle.toggled();
                            main_paned.set_position(360);
                        }
                    },
                    true => {
                        main_paned.set_position(360);
                        sidebar_stack.set_visible_child_name("function");
                        content_stack.set_visible_child_name("tables");
                        if plot_toggle.get_active() {
                            plot_toggle.set_active(false);
                            //plot_toggle.toggled();

                        }
                    }
                }
            });
        }

        {
            let main_paned = main_paned.clone();
            let table_toggle = table_toggle.clone();
            plot_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        main_paned.set_position(0);
                        if table_toggle.get_active() {
                            //table_toggle.set_active(false);
                            main_paned.set_position(360);
                        }
                    },
                    true => {
                        main_paned.set_position(360);
                        sidebar_stack.set_visible_child_name("layout");
                        content_stack.set_visible_child_name("plot");
                        if table_toggle.get_active() {
                            table_toggle.set_active(false);
                        }
                    }
                }
            });
        }
    }

    pub fn new_from_builder(builder : &Builder, window : Window) -> Self {
        //let header : HeaderBar =
        //    builder.get_object("header").unwrap();
        let tables_nb = TableNotebook::new(&builder);
        //let exec_btn : Button =
        //    builder.get_object("exec_btn").unwrap();

        let main_stack : Stack = builder.get_object("main_stack").unwrap();
        let sidebar_stack : Stack = builder.get_object("sidebar_stack").unwrap();
        let content_stack : Stack = builder.get_object("content_stack").unwrap();
        let status_stack = StatusStack::new(main_stack, content_stack.clone().upcast::<Widget>());

        let env_source = EnvironmentSource::File("".into(),"".into());
        let table_env = Rc::new(RefCell::new(TableEnvironment::new(env_source)));
        let conn_btn : Button = builder.get_object("conn_btn").unwrap();
        let popover_path = utils::glade_path("conn-popover.glade")
            .expect("Could not open glade path");
        let conn_popover = ConnPopover::new_from_glade(conn_btn, &popover_path[..]);
        let pl_da : DrawingArea = builder.get_object("plot").unwrap();
        Self::build_plots_widgets(table_env.clone(), pl_da.clone(), sidebar_stack.clone());

        // fn get_property_gtk_theme_name(&self) -> Option<GString>
        // Load icon based on theme type
        //let file_btn : FileChooserButton =
        //    builder.get_object("file_btn").unwrap();
        //let table_popover : Popover =
        //    builder.get_object("table_popover").unwrap();

        conn_popover.hook_signals(table_env.clone(), status_stack.clone());

        let query_toggle : ToggleButton =
            builder.get_object("query_toggle").unwrap();
        let sql_popover = SqlPopover::new(query_toggle.clone(), status_stack.clone(), table_env.clone());
        //sql_popover.connect_sql_load(tables_nb.clone(), table_env.clone());
        //sql_popover.connect_source_key_press(table_env.clone(), tables_nb.clone());
        //sql_popover.connect_refresh(table_env.clone(), tables_nb.clone());

        {
            let tables_nb = tables_nb.clone();
            sql_popover.connect_send_query(move ||{
                tables_nb.nb.set_sensitive(false);
                Ok(())
            });
        }

        let function_box : Box = builder.get_object("fn_box").unwrap();
        let fn_toggle = ToggleToolButton::new();
        let fn_img = Image::new_from_file("assets/icons/fn-dark.svg");
        fn_toggle.set_icon_widget(Some(&fn_img));
        //sql_popover.add_extra_toolbar(fn_toggle.clone(), function_box.clone());
        Self::build_toggles(
            builder.clone(),
            sidebar_stack.clone(),
            content_stack.clone()
        );
        //let main_paned : Paned = builder.get_object("main_paned").unwrap();
        /*let fn_toggle : ToggleButton = builder.get_object("fn_toggle").unwrap();
        let fn_popover : Popover =  builder.get_object("fn_popover").unwrap();
        {
            let fn_popover = fn_popover.clone();
            fn_toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    fn_popover.show();
                } else {
                    fn_popover.hide();
                }
            });
        }*/
        /*{
            let query_toggle = fn_toggle.clone();
            fn_popover.connect_closed(move |_popover| {
                query_toggle.set_active(false);
            });
        }*/
        /*{
            let window = window.clone();
            let fn_toggle = fn_toggle.clone();
            let main_paned = main_paned.clone();
            fn_toggle.connect_toggled(move |btn| {
                ajust_sidebar_pos(&btn, &window, &main_paned);
            });
        }

        {
            let window = window.clone();
            let fn_toggle = fn_toggle.clone();
            let main_paned = main_paned.clone();
            window.connect_check_resize(move |win| {
                //ajust_sidebar_pos(&fn_toggle, &win, &main_paned);
                println!("Resize request");
            });

            //window.connect_property_is_maximized_notify(move |win| {
            //    println!("Maximized");
            //});
        }*/

        let reg = Rc::new(NumRegistry::load().map_err(|e| { println!("{}", e); e }).unwrap());
        let funcs = reg.function_list();
        let fn_search = FunctionSearch::new(builder.clone(), reg.clone(), tables_nb.clone());
        let func_names : Vec<String> = funcs.iter().map(|f| f.name.to_string()).collect();
        println!("Function names: {:?}", func_names);
        if let Err(e) = fn_search.populate_search(func_names) {
            println!("{}", e);
        }

        //let ops_stack : Stack =
        //    builder.get_object("ops_stack").unwrap();
        /*let ws_toggle : ToggleButton =
            builder.get_object("ws_toggle").unwrap();
        //let query_toggle : ToggleButton =
        //    builder.get_object("query_toggle").unwrap();

        {
            let table_popover = table_popover.clone();
            ws_toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    table_popover.show();
                } else {
                    table_popover.hide();
                    //filter_popover.hide();
                }
            });
        }*/

        /*{
            let ws_toggle = ws_toggle.clone();
            let table_popover = table_popover.clone();
            table_popover.connect_closed(move |_popover| {
                ws_toggle.set_active(false);
            });
        }*/

        /*let new_db_dialog : FileChooserDialog =
            builder.get_object("new_db_dialog").unwrap();
        {
            let new_db_btn : Button =
                builder.get_object("new_db_btn").unwrap();
            let new_db_dialog = new_db_dialog.clone();
            new_db_btn.connect_clicked(move |_btn| {
                new_db_dialog.run();
                new_db_dialog.hide();
            });
        }*/

        /*{
            let t_env = table_env.clone();
            new_db_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            if let (Ok(mut t_env), Some(p_str)) = (t_env.try_borrow_mut(), path.to_str()) {
                                let res = t_env.update_source(
                                    EnvironmentSource::SQLite3((
                                        Some(p_str.into()),
                                        String::from(""))
                                    ),
                                    true
                                );
                                match res {
                                    Ok(_) => { },
                                    Err(s) => println!("{}", s)
                                }
                            } else {
                                println!("Could not acquire mutable reference t_env/path not convertible");
                            }
                        } else {
                            println!("No filename informed");
                        }
                    },
                    _ => { }
                }
            });
        }*/

        // let tables_nb_c = tables_nb.clone();
        //let mut table_chooser = TableChooser::new(file_btn, table_env.clone());
        /*let table_info_box : Box = builder.get_object("table_info_box").unwrap();
        {
            let table_env = table_env.clone();
            //let tables_nb = tables_nb.clone();
            table_chooser.append_cb(boxed::Box::new(move |btn| {
                /*if let Ok(t_env) = table_env.try_borrow_mut() {
                    set_tables(&t_env, &mut tables_nb.clone());
                } else {
                    println!("Unable to get reference to table env");
                }*/
                if let Ok(t_env) = table_env.try_borrow_mut() {
                    for w in table_info_box.get_children() {
                        table_info_box.remove(&w);
                    }
                    if let Some(info) = t_env.table_names_as_hash() {
                        for (table, cols) in info.iter() {
                            let exp = Expander::new(Some(&table));
                            let exp_content = Box::new(Orientation::Vertical, 0);
                            for c in cols {
                                exp_content.add(&Label::new(Some(&c.0)));
                            }
                            exp.add(&exp_content);
                            table_info_box.add(&exp);
                        }
                        table_info_box.show_all();
                    } else {
                        println!("Could not get table info as hash");
                    }
                }

            }));
        }*/

        // let sql_listener = Rc::new(RefCell::new(SqlListener::launch()));
        //let table_env_c = table_env.clone();
        // let queries_app = QueriesApp{
        //    exec_btn : exec_btn, view : view, tables_nb : tables_nb.clone(), header : header,
        //    popover : popover, table_env : table_env.clone(), query_popover : query_popover,
        //    query_toggle : query_toggle //, ws_toggle : ws_toggle
        // };
        //let queries_app_c = queries_app.clone();
        // let sql_listener_c = sql_listener.clone();
        //let table_env_c = queries_app.clone().table_env.clone();
        //let view_c = queries_app_c.view.clone();
        //let tables_nb_c = queries_app.clone().tables_nb.clone();
        // Check if there is a SQL answer before setting the widgets to sensitive again.
        //let sql_listener_c = sql_listener.clone();

        {
            //let table_env_c = table_env.clone();
            let tables_nb = tables_nb.clone();
            let fn_search = fn_search.clone();
            let f = move |t_env : &TableEnvironment| {
                //if let Ok(t_env) = table_env_c.try_borrow() {
                set_tables(&t_env, &mut tables_nb.clone(), fn_search.clone());
                tables_nb.nb.set_sensitive(true);
                Ok(())
                //} else {
                //    Err(String::from("Error retrieving reference to table environment"))
               // }
            };
            //let table_env = table_env.clone();
            sql_popover.connect_result_arrived(f);
        }

        // Query update passed to sql_popover
        /*{
            let status_stack = status_stack.clone();
            let view_c = sql_popover.clone().view.clone();
            let tables_nb_c = tables_nb.clone();
            let tbl_env_c = table_env.clone();
            let sql_popover = sql_popover.clone();
            let fn_search = fn_search.clone();
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
                                                    set_tables(&t_env, &mut tables_nb_c.clone(), fn_search.clone());
                                                    status_stack.update(Status::Ok);
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
                                tables_nb_c.nb.set_sensitive(true);
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
        }*/
        /*{
            let queries_app = queries_app.clone();
            let table_env_c = table_env.clone();
            let view_c = queries_app_c.view.clone();
            let tables_nb_c = queries_app_c.tables_nb.clone();
            queries_app.clone().exec_btn.connect_clicked(move |btn| {
                if let Ok(mut env) = table_env_c.try_borrow_mut() {
                    update_queries(&mut env, &view_c, &tables_nb_c);
                } else {
                    println!("Failed to acquire lock");
                }
            });
        }*/

        {
            let tables_nb = tables_nb.clone();
            let tbl_env = table_env.clone();
            let csv_btn : Button =
                builder.get_object("save_text_btn").unwrap();
            let save_dialog : FileChooserDialog =
                builder.get_object("save_dialog").unwrap();
            save_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            if let Some(ext) = path.as_path().extension().map(|ext| ext.to_str().unwrap_or("")) {
                                if let Ok(t) = tbl_env.try_borrow() {
                                    match ext {
                                        "db" | "sqlite" | "sqlite3" => {
                                            t.try_backup(path);
                                        },
                                        "xml" => {
                                            // save plot layout
                                        },
                                        _ => {
                                            if let Ok(mut f) = File::create(path) {
                                                let idx = tables_nb.get_page_index();
                                                if let Some(content) = t.get_text_at_index(idx) {
                                                    let _ = f.write_all(&content.into_bytes());
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    println!("Unable to get reference to table environment");
                                }
                            }
                        }
                    },
                    _ => { }
                }
            });
            csv_btn.connect_clicked(move |btn| {
                save_dialog.run();
                save_dialog.hide();
            });
        }

        Self { conn_popover, sql_popover, table_env }
    }

    //fn check_active_selection(&self) {
    //    if let Some(buf) = self.view.get_buffer() {

            /*if buf.get_has_selection() {
                self.exec_btn.set_sensitive(true);
            } else {
                self.exec_btn.set_sensitive(false);
            }*/

       // }
    //}

    /*fn run_query(popover : &mut ConnPopover, buffer : &TextBuffer) {
        let mut query = String::new();
        if let Some((from,to,)) = buffer.get_selection_bounds() {
            if let Some(txt) = from.get_text(&to) {
                query = txt.to_string();
            }
        }
        if query.len() > 0 {
            popover.parse_sql(query);
            popover.try_run_all();
        }
        // TODO Update notebook here with query results
    }*/

}

fn build_func_menu(builder : Builder) {
    let search_entry : Entry =
            builder.get_object("fn_search_entry").unwrap();
        let completion : EntryCompletion =
            builder.get_object("fn_completion").unwrap();
        completion.set_text_column(0);
        completion.set_minimum_key_length(1);
}

fn build_ui(app: &gtk::Application) {
    let path = utils::glade_path("gtk-queries-funcs-popover.glade").expect("Failed to load glade file");
    let builder = Builder::new_from_file(path);
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");

    let queries_app = QueriesApp::new_from_builder(&builder, win.clone());

    {
        let toggle_q = queries_app.sql_popover.query_toggle.clone();
        //let toggle_w = queries_app.ws_toggle.clone();
        let view = queries_app.sql_popover.view.clone();
        win.connect_key_release_event(move |win, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::MOD1_MASK {
                if ev_key.get_keyval() == key::q {
                    if toggle_q.get_active() {
                        toggle_q.set_active(false);
                    } else {
                        toggle_q.set_active(true);
                        view.grab_focus();
                    }
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == key::w {
                    //if toggle_w.get_active() {
                    //    toggle_w.set_active(false);
                    //} else {
                    //    toggle_w.set_active(true);
                    //}
                    return glib::signal::Inhibit(true)
                }
                return glib::signal::Inhibit(false)
            } else {
                glib::signal::Inhibit(false)
            }
        });
    }

    win.set_application(Some(app));

    win.show_all();
}

fn main() {
    gtk::init();

    // Required for GtkSourceView initialization from glade
    let _ = View::new();

    let app = gtk::Application::new(
        Some("com.github.limads.gtk-queries"),
        Default::default())
    .expect("Could not initialize Gtk");

    app.connect_activate(|app| {
        build_ui(app);
    });

    app.run(&args().collect::<Vec<_>>());
}

// Change GtkSourceView on file set
/*{
    let view = view.clone();
    // let new_file = new_file.clone();
    // let header = header.clone();
    // let unsaved_dialog = unsaved_dialog.clone();
    let unsaved_changes = unsaved_changes.clone();
    file_btn.connect_file_set(move |btn| {
        let buffer = view.get_buffer();
        let new_file = new_file.try_borrow_mut();
        let unsaved = unsaved_changes.try_borrow_mut();
        match (buffer, new_file, unsaved) {
            (Some(mut buf), Ok(mut new_f), Ok(mut unsaved)) => {
                let from = buf.get_start_iter();
                let to = buf.get_end_iter();
                let empty_buf =  from == to;
                if (*new_f && !empty_buf) || *unsaved {
                    match unsaved_dialog.run() {
                        ResponseType::Other(0) => {
                            unsaved_dialog.hide();
                        },
                        ResponseType::Other(1) => {
                            buf.set_text("");
                            let ok = QueriesApp::load_file_to_buffer(
                                btn.clone(),
                                buf,
                                header.clone(),
                                *new_f,
                                *unsaved
                            ).is_ok();
                            if ok {
                                *new_f = false;
                                *unsaved = false;
                            }
                            unsaved_dialog.hide();
                        },
                        _ => { }
                    }
                }
            },
            _ => { println!("Unavailable reference"); }
        }
    });*/

// SourceView key release
/*{
let exec_btn = queries_app.exec_btn.clone();
let queries_app = queries_app.clone();
queries_app.clone().view.connect_key_release_event(move |view, ev| {
    queries_app.check_active_selection();
    if let Some(buf) = queries_app.view.get_buffer() {
        let from = &buf.get_start_iter();
        let to = &buf.get_end_iter();
        let old = queries_app.old_source_content.borrow();
        let unsaved = queries_app.unsaved_changes.borrow();
        if let Some(txt) = buf.get_text(from, to,true) {
            if *unsaved && txt != *old {
                let mut subtitle : String =
                    queries_app.header.get_subtitle()
                    .and_then(|s| Some(s.to_string()) )
                    .unwrap_or("".into());
                subtitle += "*";
                queries_app.header.set_subtitle(
                    Some(&subtitle[..]));
            }
        }
    }
glib::signal::Inhibit(false)
});
}*/

// Button release on GtkSourceView
/*
{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_button_release_event(move |view, ev| {
        queries_app.check_active_selection();
        glib::signal::Inhibit(false)
    });
}
*/

// Key press on GtkSourceView
/*{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_key_press_event(move |view, ev_key| {
        // check gdkkeysyms.h
        println!("{:?}", ev_key.get_keyval());

        if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK {
            if ev_key.get_keyval() == 115 {
                println!("must save now");
            }
        }

        match ev_key.get_keyval() {
            // s, i.e. CTRL+s because plain s is
            // captured and inhibited
            //if
            0x073 => {
                // queries_app.try_save_file();
            },
            // space
            0x020 => {
            }
            _ => { }
        };
        glib::signal::Inhibit(false)
    });
}*/

