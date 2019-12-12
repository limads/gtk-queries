use gtk::*;
use gio::prelude::*;
use std::env::args;
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
use tables::{EnvironmentSource, TableEnvironment, button::TableChooser};
use tables::table_widget::*;
mod conn_popover;
use conn_popover::*;
use tables::table_notebook::*;
use sourceview::*;

#[derive(Clone)]
pub struct QueriesApp {
    exec_btn : Button,
    view : sourceview::View,
    tables_nb : TableNotebook,
    // file_btn : FileChooserButton,
    header : HeaderBar,
    // unsaved_dialog : Dialog,
    // new_file : Rc<RefCell<bool>>,
    // unsaved_changes : Rc<RefCell<bool>>,
    // save_dialog : Dialog,
    popover : Rc<RefCell<ConnPopover>>,
    table_env : Rc<RefCell<TableEnvironment>>,
    // old_source_content : Rc<RefCell<String>>
}

impl QueriesApp {

    pub fn new_from_builder(builder : &Builder) -> Self {
        let header : HeaderBar =
            builder.get_object("header").unwrap();
        let tables_nb = TableNotebook::new(&builder);
        let exec_btn : Button =
            builder.get_object("exec_btn").unwrap();
        let view : sourceview::View =
            builder.get_object("query_source").unwrap();
        let lang_manager = LanguageManager::get_default().unwrap();
        let buffer = view.get_buffer().unwrap()
            .downcast::<sourceview::Buffer>().unwrap();
        let lang = lang_manager.get_language("sql").unwrap();
        buffer.set_language(Some(&lang));

        let conn_btn : Button = builder.get_object("conn_btn").unwrap();
        let popover = ConnPopover::new_from_glade(
            conn_btn,"assets/gui/conn-popover.glade");
        popover.hook_signals();
        let popover = Rc::new(RefCell::new(popover));

        let env_source = EnvironmentSource::File("".into(),"".into());
        let table_env = TableEnvironment::new(env_source);
        let table_env = Rc::new(RefCell::new(table_env));

        let queries_app = QueriesApp{
            exec_btn, view, tables_nb, header,
            popover, table_env
        };

        {
            let queries_app = queries_app.clone();
            queries_app.clone().exec_btn.connect_clicked(move |btn| {
                let mut popover =
                    queries_app.popover.try_borrow_mut().unwrap();
                let mut table_env =
                    queries_app.table_env.try_borrow_mut().unwrap();
                if let Some(buffer) = queries_app.view.get_buffer() {
                    QueriesApp::run_query(&mut popover, &buffer);
                    popover.mark_all_valid();
                    let valid_queries = popover.get_valid_queries();
                    match table_env.update_content_from_queries(valid_queries) {
                        Err(e) => { println!("{}", e); },
                        _ => {}
                    }
                }
                let nb = &queries_app.tables_nb;
                nb.clear();
                if popover.queries.is_empty() {
                    nb.add_page("application-exit",
                        None, Some("No queries"), None);
                } else {
                    for q in popover.queries.iter() {
                        match &q.err_msg {
                            None => {
                                let rows = q.as_rows();
                                let nrows = rows.len();
                                if let Some(cols) = rows.get(0) {
                                    let ncols = rows.len();
                                    let name = format!("({} x {})",
                                        nrows, ncols);
                                    nb.add_page("network-server-symbolic",
                                    Some(&name[..]), None, Some(rows));
                                } else {
                                    print!("Empty content");
                                }

                            },
                            Some(msg) => {
                                nb.add_page("application-exit",
                                    None, Some(&msg[..]), None);
                            }
                        }
                    }
                }
            });
        }
        queries_app
    }

    fn check_active_selection(&self) {
        if let Some(buf) = self.view.get_buffer() {
            if buf.get_has_selection() {
                self.exec_btn.set_sensitive(true);
            } else {
                self.exec_btn.set_sensitive(false);
            }
        }
    }

    fn run_query(popover : &mut ConnPopover, buffer : &TextBuffer) {
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
    }

}

fn build_ui(app: &gtk::Application) {
    let builder = Builder::new_from_file("assets/gui/gnome-queries.glade");
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");
    win.set_application(Some(app));
    let queries_app = QueriesApp::new_from_builder(&builder);
    win.show_all();
}

fn main() {
    gtk::init();

    // Required for GtkSourceView initialization from glade
    let _ = View::new();

    let app = gtk::Application::new(
        Some("com.github.limads.gtk-plots"),
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

