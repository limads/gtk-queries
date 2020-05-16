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
use gtk_queries::tables::{source::EnvironmentSource, environment::TableEnvironment, environment::EnvironmentUpdate, sql::SqlListener};
use gtk_queries::conn_popover::*;
use sourceview::*;
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use gtk_queries::{utils, table_widget::TableWidget, table_notebook::TableNotebook };
//use nlearn::table::Table;
use gtk_queries::tables::table::*;
use gtk_queries::status_stack::*;
use gtk_queries::sql_popover::*;
use gtk_queries::functions::function_search::*;
use gtk_queries::functions::num_function::*;
use gdk::prelude::*;
use gtkplotview::plot_view::PlotView;
use gtk_queries::plots::save_widgets;
use gtk_queries::plots::layout_menu::PlotSidebar;
use gtk_queries::upload_popover::*;

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
    fn_popover : Popover,
    table_env : Rc<RefCell<TableEnvironment>>,
    tables_nb : TableNotebook,
    main_paned : Paned,
    table_toggle : ToggleButton,
    plot_toggle : ToggleButton
    //query_toggle : ToggleButton //,
    //ws_toggle : ToggleButton

    // old_source_content : Rc<RefCell<String>>
}

/*fn adjust_sidebar_pos(btn : &ToggleButton, window : &Window, main_paned : &Paned) {
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
}*/

impl QueriesApp {

    fn build_plots_widgets(
        builder : Builder,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        pl_da : DrawingArea,
        //sidebar_stack : Stack,
        status_stack : StatusStack,
        plot_toggle : ToggleButton
    ) -> PlotSidebar {
        //let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let pl_view = PlotView::new_with_draw_area(
            "assets/plot_layout/layout-single.xml", pl_da.clone());
        save_widgets::build_save_widgets(&builder, pl_view.clone());
        let sidebar = PlotSidebar::new(
            builder.clone(),
            pl_view.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            status_stack,
            plot_toggle.clone()
        );
        //sidebar_stack.add_named(&sidebar.layout_stack, "layout");
        sidebar
    }

    fn connect_toggles(
        table_toggle : ToggleButton,
        plot_toggle : ToggleButton,
        builder : Builder,
        main_paned : Paned,
        //sidebar_stack : Stack,
        content_stack : Stack,
        status_stack : StatusStack,
        plot_sidebar : PlotSidebar
    ) -> (ToggleButton, ToggleButton) {
        //main_paned.add1(&plot_sidebar.sidebar_box);
        //main_paned.show_all();
        //plot_sidebar.sidebar_box.show_all();

        {
            let main_paned = main_paned.clone();
            let plot_toggle = plot_toggle.clone();
            //let sidebar_stack = sidebar_stack.clone();
            let content_stack = content_stack.clone();
            let plot_sidebar = plot_sidebar.clone();
            let status_stack = status_stack.clone();
            table_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        //content_stack.set_visible_child_name("plot");
                        if !plot_sidebar.layout_loaded() {
                            status_stack.try_show_alt_or_connected();
                        }
                        main_paned.set_position(0);
                        if plot_toggle.get_active() {
                            //plot_toggle.set_active(false);
                            plot_toggle.toggled();
                            main_paned.set_position(460);
                        }
                    },
                    true => {
                        content_stack.set_visible_child_name("tables");
                        status_stack.try_show_alt_or_connected();
                        main_paned.set_position(460);
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
                            table_toggle.toggled();
                            main_paned.set_position(360);
                        }
                    },
                    true => {
                        main_paned.set_position(360);
                        content_stack.set_visible_child_name("plot");
                        if let Ok(pl_view) = plot_sidebar.pl_view.try_borrow() {
                            pl_view.redraw();
                        } else {
                            println!("Failed to acquire lock over plot")
                        }

                        //if !plot_sidebar.layout_loaded() {
                        //    status_stack.show_curr_status();
                        //} //else {
                        //
                        //}
                        if table_toggle.get_active() {
                            table_toggle.set_active(false);
                            //table_toggle.toggled();
                        }
                        if plot_sidebar.layout_loaded() {
                            status_stack.try_show_alt();
                        } else {
                            status_stack.show_curr_status();
                        }

                    }
                }
            });
        }
        (table_toggle, plot_toggle)
    }

    pub fn new_from_builder(builder : &Builder, window : Window) -> Self {
        //let header : HeaderBar =
        //    builder.get_object("header").unwrap();
        let main_paned : Paned =  builder.get_object("main_paned").unwrap();
        let tables_nb = TableNotebook::new(&builder);
        //let exec_btn : Button =
        //    builder.get_object("exec_btn").unwrap();

        let main_stack : Stack = builder.get_object("main_stack").unwrap();
        //let sidebar_stack : Stack = builder.get_object("sidebar_stack").unwrap();
        let content_stack : Stack = builder.get_object("content_stack").unwrap();
        let status_stack = StatusStack::new(builder.clone(), main_stack, content_stack.clone().upcast::<Widget>());

        let table_toggle : ToggleButton = builder.get_object("table_toggle").unwrap();
        let plot_toggle : ToggleButton = builder.get_object("plot_toggle").unwrap();

        let env_source = EnvironmentSource::File("".into(),"".into());
        let table_env = Rc::new(RefCell::new(TableEnvironment::new(env_source)));
        let conn_btn : Button = builder.get_object("conn_btn").unwrap();
        let popover_path = utils::glade_path("conn-popover.glade")
            .expect("Could not open glade path");
        let conn_popover = ConnPopover::new_from_glade(builder.clone(), conn_btn, &popover_path[..]);
        let pl_da : DrawingArea = builder.get_object("plot").unwrap();
        let sidebar = Self::build_plots_widgets(
            builder.clone(),
            table_env.clone(),
            tables_nb.clone(),
            pl_da.clone(),
            status_stack.clone(),
            plot_toggle.clone()
        );

        let upload_popover = UploadPopover::new(builder.clone(), tables_nb.clone());
        // fn get_property_gtk_theme_name(&self) -> Option<GString>
        // Load icon based on theme type
        //let file_btn : FileChooserButton =
        //    builder.get_object("file_btn").unwrap();
        //let table_popover : Popover =
        //    builder.get_object("table_popover").unwrap();

        let sql_popover = SqlPopover::new(
            builder.clone(),
            status_stack.clone(),
            table_env.clone()
        );
        conn_popover.hook_signals(
            table_env.clone(),
            tables_nb.clone(),
            status_stack.clone(),
            sql_popover.clone(),
            sidebar.clone()
        );

        //let query_toggle : ToggleButton =
        //    builder.get_object("query_toggle").unwrap();

        //sql_popover.connect_sql_load(tables_nb.clone(), table_env.clone());
        //sql_popover.connect_source_key_press(table_env.clone(), tables_nb.clone());
        //sql_popover.connect_refresh(table_env.clone(), tables_nb.clone());

        {
            let tables_nb = tables_nb.clone();
            let status_stack = status_stack.clone();
            sql_popover.connect_send_query(move |res| {
                match res {
                    Ok(_) => {
                        tables_nb.nb.set_sensitive(false);
                    },
                    Err(e) => {
                        status_stack.update(Status::SqlErr(e));
                    }
                }
                Ok(())
            });
        }

        //let function_box : Box = builder.get_object("fn_box").unwrap();
        let fn_toggle = ToggleToolButton::new();
        let fn_img = Image::new_from_file("assets/icons/fn-dark.svg");
        fn_toggle.set_icon_widget(Some(&fn_img));
        //sql_popover.add_extra_toolbar(fn_toggle.clone(), function_box.clone());
        Self::connect_toggles(
            table_toggle.clone(),
            plot_toggle.clone(),
            builder.clone(),
            main_paned.clone(),
            //sidebar_stack.clone(),
            content_stack.clone(),
            status_stack.clone(),
            sidebar.clone()
        );
        let fn_popover : Popover = builder.get_object("fn_popover").unwrap();
        let reg = Rc::new(NumRegistry::load().map_err(|e| { println!("{}", e); e }).unwrap());
        let funcs = reg.function_list();
        let fn_search = FunctionSearch::new(
            builder.clone(),
            reg.clone(),
            tables_nb.clone(),
            fn_popover.clone(),
            sidebar.clone(),
            table_env.clone()
        );
        let func_names : Vec<String> = funcs.iter().map(|f| f.name.to_string()).collect();
        println!("Function names: {:?}", func_names);
        if let Err(e) = fn_search.populate_search(func_names) {
            println!("{}", e);
        }

        {
            //let table_env_c = table_env.clone();
            let tables_nb = tables_nb.clone();
            let fn_search = fn_search.clone();
            let fn_popover = fn_popover.clone();
            let sidebar = sidebar.clone();
            let status_stack = status_stack.clone();
            let f = move |t_env : &TableEnvironment, update : &EnvironmentUpdate| {
                //if let Ok(t_env) = table_env_c.try_borrow() {
                utils::set_tables(
                    &t_env,
                    &mut tables_nb.clone(),
                    fn_search.clone(),
                    sidebar.clone(),
                    fn_popover.clone()
                );
                // TODO update all mappings to NULL data and set mappings as insensitive
                // if new table output is different than old table output.
                match update {
                    EnvironmentUpdate::Refresh => {
                        sidebar.update_all_mappings(
                            &t_env,
                            status_stack.clone()
                        );
                    },
                    _ => {
                        sidebar.clear_all_mappings();
                    }
                }
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

        /*let xml_save_dialog : FileChooserDialog =
                builder.get_object("xml_save_dialog").unwrap();
        {

            xml_save_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {

                        }
                    },
                    _ => { }
                }
            });
        }*/

        {
            let pl_view = sidebar.pl_view.clone();
            let tables_nb = tables_nb.clone();
            let tbl_env = table_env.clone();
            let csv_btn : Button =
                builder.get_object("save_text_btn").unwrap();
            let save_dialog : FileChooserDialog =
                builder.get_object("save_dialog").unwrap();
            save_dialog.connect_response(move |dialog, resp| {
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
                                            if let Ok(pl) = pl_view.try_borrow() {
                                                if let Ok(mut f) = File::create(path) {
                                                    let content = pl.plot_group.get_layout_as_text();
                                                    let _ = f.write_all(&content.into_bytes());
                                                } else {
                                                    println!("Unable to create file");
                                                }
                                            } else {
                                                println!("Unable to retrieve reference to plot");
                                            }
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

        Self { conn_popover, sql_popover, main_paned, table_env, fn_popover, tables_nb, table_toggle, plot_toggle }
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

pub fn switch_paned(paned : Paned, show : bool) {
    if paned.get_position() == 0 {
        if show {
            paned.set_position(360);
        }
    } else {
        if !show {
            paned.set_position(0);
        }
    }
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
    let path = utils::glade_path("gtk-queries.glade").expect("Failed to load glade file");
    let builder = Builder::new_from_file(path);
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");

    let queries_app = QueriesApp::new_from_builder(&builder, win.clone());

    {
        //let toggle_q = queries_app.sql_popover.query_toggle.clone();
        let conn_popover = queries_app.conn_popover.clone();
        let fn_popover = queries_app.fn_popover.clone();
        let tables_nb = queries_app.tables_nb.clone();
        //let toggle_w = queries_app.ws_toggle.clone();
        let view = queries_app.sql_popover.view.clone();
        let main_paned = queries_app.main_paned.clone();
        let table_toggle = queries_app.table_toggle.clone();
        let plot_toggle = queries_app.plot_toggle.clone();
        let sql_stack : Stack = queries_app.sql_popover.sql_stack.clone();
        win.connect_key_release_event(move |win, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::MOD1_MASK {
                if ev_key.get_keyval() == key::q {
                    if conn_popover.popover.get_visible() {
                        conn_popover.popover.hide();
                    }
                    if table_toggle.get_active() == false {
                        table_toggle.set_active(true);
                    }
                    sql_stack.set_visible_child_name("source");
                    if !view.get_realized() {
                        view.realize();
                    }
                    view.grab_focus();
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == key::w {
                    if plot_toggle.get_active() == false {
                        plot_toggle.set_active(true);
                    }
                }
                if ev_key.get_keyval() == key::c {
                    conn_popover.popover.show();
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == key::f {
                    if tables_nb.selected_cols().len() > 0 {
                        if !fn_popover.is_visible() {
                            fn_popover.show();
                        } else {
                            fn_popover.hide();
                        }
                    }
                }
                if ev_key.get_keyval() == key::l {

                }
                if ev_key.get_keyval() == key::Escape {
                    fn_popover.hide();
                    conn_popover.popover.hide();
                    table_toggle.set_active(false);
                    plot_toggle.set_active(false);
                    tables_nb.unselect_at_table();
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

// TODO change back to table when user goes to connected status message at the plot screen.

// cp queries.desktop /usr/share/applications
// cp assets/icons/queries.svg /usr/share/icons/hicolor/128x128/apps
// cp target/debug/queries /usr/bin/
fn main() {
    gtk::init();

    // Required for GtkSourceView initialization from glade
    let _ = View::new();

    let app = gtk::Application::new(
        Some("com.github.limads.queries"),
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

