// use crate::tables::sql::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::*;
use gtk::prelude::*;
//use postgres::{Connection, TlsMode};
use std::collections::HashMap;
use crate::tables::environment::TableEnvironment;
// use crate::tables::sql::{SqlListener};
use crate::tables::source::EnvironmentSource;
// use gtk::prelude::*;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use crate::tables::table::*;
use crate::status_stack::*;
use crate::sql_popover::SqlPopover;
use crate::plots::layout_menu::PlotSidebar;
use crate::table_notebook::*;

#[derive(Clone)]
pub struct ConnPopover {
    btn : gtk::Button,
    pub popover : gtk::Popover,
    entries : [gtk::Entry; 4],
    conn_switch : Switch,
    db_file_btn : Button,
    db_file_dialog : FileChooserDialog,
    //query_file_dialog : FileChooserDialog,
    db_file_img : Image,
    db_path : Rc<RefCell<Vec<PathBuf>>>,
    //query_update_combo : ComboBoxText,
    //query_upload_btn : Button,
    //query_update_btn : Button
}

/*
Widgets that go together can be represented inside
small structs. Structs can then derive clone (which
will clone widgets recursively).
*/
/*#[derive(Clone)]
struct Conn {
    switch : Switch,
    conn : Option<Connection>
}*/

/*fn build_ui(app: &gtk::Application) {
    let builder = Builder::new_from_file("assets/gui/gtk-tables-2.glade");
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");
    win.set_application(Some(app));
    let queries_app = QueriesApp::new_from_builder(&builder, win.clone());
    win.show_all();
}*/

impl ConnPopover {

    /*fn new(btn : gtk::Button, popover : gtk::Popover) -> ConnPopover {
        let host_entry : gtk::Entry =
        builder.get_object("host_entry");
       ConnPopover{btn, popover, conn}
    }*/
    /* Load popover from a path to a glade file */
    /* It is important to notice ConnPopover will take ownership
    of btn here */
    pub fn new_from_glade(
        builder : Builder,
        btn : gtk::Button,
        _path : &str
    )
        //conn : &'a mut Option<Connection>,
        /*conn_switch : &'a mut gtk::Switch)*/ -> ConnPopover {
        //let builder = Builder::new_from_file(path);
        let popover : gtk::Popover =
            builder.get_object("conn_popover").unwrap();
        popover.set_relative_to(Some(&btn));
        let host_entry : Entry =
            builder.get_object("host_entry").unwrap();
        let user_entry : Entry =
            builder.get_object("user_entry").unwrap();
        let password_entry : Entry =
            builder.get_object("password_entry").unwrap();
        let db_entry : Entry =
            builder.get_object("database_entry").unwrap();
        let entries = [host_entry, user_entry, password_entry, db_entry];

        let conn_switch : Switch =
            builder.get_object("conn_switch").unwrap();
        let db_file_dialog : FileChooserDialog =
            builder.get_object("db_file_dialog").unwrap();
        //let query_file_dialog : FileChooserDialog =
        //    builder.get_object("query_file_dialog").unwrap();
        let db_file_btn : Button =
            builder.get_object("db_file_btn").unwrap();
        //let query_upload_btn : Button =
        //    builder.get_object("query_upload_btn").unwrap();
        //let query_update_btn : Button =
        //    builder.get_object("query_update_btn").unwrap();
        let db_file_img : Image =
            builder.get_object("db_file_img").unwrap();
        //let query_update_combo : ComboBoxText =
        //    builder.get_object("query_update_combo").unwrap();

        {
            let db_file_dialog = db_file_dialog.clone();
            db_file_btn.connect_clicked(move |_btn| {
                println!("Here");
                db_file_dialog.run();
                db_file_dialog.hide();
            });
        }

        /*{
            let query_file_dialog = query_file_dialog.clone();
            query_upload_btn.connect_clicked(move |_btn| {
                println!("Here");
                query_file_dialog.run();
                query_file_dialog.hide();
            });
        }*/

        /*{
            let query_upload_btn = query_upload_btn.clone();
            let query_update_btn = query_update_btn.clone();
            let query_file_dialog = query_file_dialog.clone();
            query_update_combo.connect_changed(move |combo|{
                if let Some(txt) = combo.get_active_text().map(|txt| txt.to_string()) {
                    match &txt[..] {
                        "Query off" => {
                            query_upload_btn.set_sensitive(false);
                            query_update_btn.set_sensitive(false);
                            query_file_dialog.unselect_all();
                            // TODO if auto-update, stop here.
                        },
                        other => {
                            query_upload_btn.set_sensitive(true);
                            query_update_btn.set_sensitive(true);
                            match other {
                                "1 Second" => { },
                                "5 Seconds" => { },
                                "10 Seconds" => { },
                                _ => { }
                            }
                        },
                    }
                }
            });
        }*/
        let db_path = Rc::new(RefCell::new(Vec::new()));
        ConnPopover{
            btn,
            popover,
            entries,
            conn_switch,
            db_file_btn,
            db_file_dialog,
            db_path,
            db_file_img,
            //query_update_combo,
            //query_upload_btn,
            //query_update_btn,
            //query_file_dialog
        }
    }

    fn try_remote_connection(
        conn_popover : &ConnPopover,
        t_env : &mut TableEnvironment
    ) -> Result<(), String> {
        if t_env.is_engine_active() {
            return Err(format!("Invalid connection state"));
        }
        if conn_popover.check_entries_clear() {
            return Err(format!("Invalid connection parameters"));
        }
        match ConnPopover::generate_conn_str(&conn_popover.entries) {
            Ok(conn_str) => {
                let res = t_env.update_source(
                    EnvironmentSource::PostgreSQL((conn_str, "".into())),
                    true
                );
                match res {
                    Ok(_) => {
                        conn_popover.set_db_loaded_mode();
                        Ok(())
                    },
                    Err(e) => {
                        Err(format!("{}", e))
                    }
                }
            },
            Err(err_str) => {
                Err(err_str)
            }
        }
    }

    fn try_local_connection(
        conn_popover : &ConnPopover,
        opt_path : Option<PathBuf>,
        t_env : &mut TableEnvironment
    ) -> Result<(), String> {
        if t_env.is_engine_active() {
            return Err(format!("Invalid connection state"));
        }
        let source = EnvironmentSource::SQLite3((opt_path.clone(), String::new()));
        if let Err(e) = t_env.update_source(source, true) {
            println!("{}", e);
            return Err(e);
        }
        let conn_name = match &opt_path {
            Some(path) => {
                if let Some(str_path) = path.to_str() {
                    str_path
                } else {
                    "(Invalid UTF-8 path)"
                }
            }
            None => "(In-memory database)"
        };
        conn_popover.entries[3].set_text(conn_name);
        conn_popover.set_db_loaded_mode();
        Ok(())
    }

    fn set_db_loaded_mode(&self) {
        self.entries.iter().for_each(|entry| entry.set_sensitive(false) );
        self.db_file_btn.set_sensitive(false);
        //self.query_update_combo.set_active_id(Some("0"));
        //self.query_update_combo.set_sensitive(true);
    }

    fn set_non_db_mode(&self) {
        self.entries.iter().for_each(|entry| entry.set_sensitive(true) );
        self.db_file_btn.set_sensitive(true);
        //self.query_update_combo.set_active_id(Some("0"));
        //self.query_update_combo.set_sensitive(false);
        //self.query_upload_btn.set_sensitive(false);
        //self.query_update_btn.set_sensitive(false);
        if let Ok(mut db_p) = self.db_path.try_borrow_mut() {
            *db_p = Vec::new();
        } else {
            println!("Could not get mutable reference to db path");
        }
    }

    fn disconnect_with_delay(
        _switch : Switch
    ) {
        //let switch = switch.clone();
        gtk::timeout_add(160, move || {
            //&switch.set_state(false);
            glib::Continue(false)
        });
    }

    fn clear_session(
        sql_popover : SqlPopover,
        plot_sidebar : PlotSidebar,
        table_notebook : TableNotebook,
        t_env : &mut TableEnvironment
    ) {
        sql_popover.set_active(false);
        plot_sidebar.set_active(false);
        table_notebook.clear();
        plot_sidebar.clear();
        //if let Ok(mut t_env) = table_env.try_borrow_mut() {
        t_env.clear();
        t_env.clear_queries();
        //} else {
        //    println!("Unable to retrieve lock ove table environment");
        //}
    }

    pub fn hook_signals(
        &self,
        table_env : Rc<RefCell<TableEnvironment>>,
        table_notebook : TableNotebook,
        status : StatusStack,
        sql_popover : SqlPopover,
        plot_sidebar : PlotSidebar
    ) {
        let conn_popover = self.clone();
        self.conn_switch.connect_state_set(move |switch, state| {
            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                if state {
                    if let Ok(db_path) = conn_popover.db_path.try_borrow() {
                        match (db_path.len(), conn_popover.check_entries_clear()) {
                            (0, true) => {
                                 match Self::try_local_connection(&conn_popover, None, &mut t_env) {
                                    Ok(_) => status.update(Status::Connected),
                                    Err(e) => {
                                        status.update(Status::ConnectionErr(e));
                                        Self::disconnect_with_delay(switch.clone());
                                    }
                                 }
                            },
                            (0, false) => {
                                match Self::try_remote_connection(&conn_popover, &mut t_env) {
                                    Ok(_) => status.update(Status::Connected),
                                    Err(e) => {
                                        status.update(Status::ConnectionErr(e));
                                        Self::disconnect_with_delay(switch.clone());
                                    }
                                }
                            },
                            (1, true) => {
                                println!("{:?}", db_path);
                                if let Some(ext) = db_path[0].extension().map(|ext| ext.to_str()) {
                                    match ext {
                                        Some("csv") | Some("txt") => {
                                            match Self::try_local_connection(&conn_popover, None, &mut t_env) {
                                                Ok(_) => status.update(Status::Connected),
                                                Err(e) => {
                                                    status.update(Status::ConnectionErr(e));
                                                    Self::disconnect_with_delay(switch.clone());
                                                }
                                            }
                                            Self::upload_csv(db_path[0].clone(), &mut t_env, status.clone(), switch.clone());
                                            // Self::select_all_tables(&mut t_env);
                                        },
                                        _ => {
                                            match Self::try_local_connection(&conn_popover, Some(db_path[0].clone()), &mut t_env) {
                                                Ok(_) => status.update(Status::Connected),
                                                Err(e) => {
                                                    status.update(Status::ConnectionErr(e));
                                                    Self::disconnect_with_delay(switch.clone());
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    match Self::try_local_connection(&conn_popover, None, &mut t_env) {
                                        Ok(_) => status.update(Status::Connected),
                                        Err(e) => {
                                            status.update(Status::ConnectionErr(e));
                                            Self::disconnect_with_delay(switch.clone());
                                        }
                                    }
                                }
                            },
                            (_, true) => {
                                match Self::try_local_connection(&conn_popover, None, &mut t_env) {
                                    Ok(_) => status.update(Status::Connected),
                                    Err(e) => {
                                        status.update(Status::ConnectionErr(e));
                                        Self::disconnect_with_delay(switch.clone());
                                    }
                                }
                                for p in db_path.iter() {
                                    Self::upload_csv(p.clone(), &mut t_env, status.clone(), switch.clone());
                                }
                                // Self::select_all_tables(&mut t_env);
                            },
                            _ => {
                                println!("Invalid connection mode");
                            }
                        }
                    } else {
                        println!("Could not acquire lock over DB path");
                    }
                } else {
                    // Disable remote connection
                    if t_env.is_engine_active() {
                        t_env.disable_engine();
                    }
                    conn_popover.set_non_db_mode();
                    conn_popover.clear_entries();
                    status.update(Status::Disconnected);
                    Self::clear_session(
                        sql_popover.clone(),
                        plot_sidebar.clone(),
                        table_notebook.clone(),
                        &mut t_env
                    );
                }
            } else {
                println!("Could not acquire lock over table environment");
            }
            if let Some(status) = status.get_status() {
                match status {
                    Status::Connected => {
                        sql_popover.set_active(true);
                        plot_sidebar.set_active(true);
                    },
                    _ => {
                        if let Ok(mut t_env) = table_env.try_borrow_mut() {
                            Self::clear_session(
                                sql_popover.clone(),
                                plot_sidebar.clone(),
                                table_notebook.clone(),
                                &mut t_env
                            );
                        } else {
                            println!("Failed to acquire lock over table environment");
                        }
                    }
                }
            }
            Inhibit(false)
        });

        {
            let conn_popover = self.clone();
            self.db_file_dialog.connect_response(move |dialog, resp| {
                match resp {
                    ResponseType::Other(1) => {
                        let fnames = dialog.get_filenames();
                        if let Ok(mut db_p) = conn_popover.db_path.try_borrow_mut() {
                            if fnames.len() >= 1 {
                                conn_popover.clear_entries();
                                db_p.clear();
                                db_p.extend(fnames.clone());
                                let path = &fnames[0];
                                let db_name = if let Some(ext) = path.extension().map(|ext| ext.to_str()) {
                                    match ext {
                                        Some("csv") | Some("txt") => {
                                            "In-memory"
                                        },
                                        Some("db") | Some("sqlite3") | Some("sqlite") => {
                                            if let Some(path_str) = path.to_str() {
                                                path_str
                                            } else {
                                                "(Non UTF-8 path)"
                                            }
                                        },
                                        _ => {
                                            "(Unknown extension)"
                                        }
                                    }
                                } else {
                                    "(Unknown extension)"
                                };
                                conn_popover.entries[3].set_text(db_name);
                            }
                        } else {
                            println!("Failed to get lock over db path");
                        }
                    },
                    _ => { }
                }
            });
        }

        let popover = self.popover.clone();
        self.btn.connect_clicked(move |_| {
            popover.show();
        });
    }

    fn check_entries_clear(&self) -> bool {
        for entry in self.entries.iter().take(3) {
            if let Some(txt) = entry.get_text().map(|t| t.to_string()) {
                if !txt.is_empty() {
                    return false;
                }
            }
        }
        true
    }

    fn clear_entries(&self) {
        for entry in self.entries.iter() {
            entry.set_text("");
        }
    }

    /*/// Assume there is a 1:1 correspondence between table names
    /// and tables at the database. Select all rows of all tables.
    fn select_all_tables(t_env : &mut TableEnvironment) {
        /*let names : Vec<_> =  t_env.all_tables().iter()
            .map(|t| t.name.clone().unwrap_or("tbl".into()) ).collect();
        for name in names {
            let sql = format !("select * from {};", name);
            t_env.send_query(sql);
        }*/
    }*/

    fn upload_csv(path : PathBuf, t_env : &mut TableEnvironment, status_stack : StatusStack, switch : Switch) {
        if let Some(name) = path.clone().file_name().map(|n| n.to_str()) {
            if let Some(name) = name.map(|n| n.split('.').next()) {
                if let Some(name) = name {
                    let mut content = String::new();
                    if let Ok(mut f) = File::open(path) {
                        if let Ok(_) = f.read_to_string(&mut content) {
                            let t = Table::new_from_text(content);
                            match t {
                                Ok(t) => {
                                    match t.sql_string(name) {
                                        Ok(sql) => {
                                            // TODO there is a bug here when the user executes the first query, because
                                            // the first call to indle callback will retrieve the create/insert statements,
                                            // not the actual user query.
                                            if let Err(e) = t_env.prepare_and_send_query(sql) {
                                                status_stack.update(Status::SqlErr(e));
                                            }
                                        },
                                        Err(e) =>  {
                                            status_stack.update(Status::SqlErr(
                                                format!("Failed to generate SQL: {}", e)
                                            ));
                                            Self::disconnect_with_delay(switch.clone());
                                        }
                                    }
                                },
                                Err(e) => {
                                    status_stack.update(Status::SqlErr(
                                        format!("Could not generate SQL: {}", e))
                                    );
                                    Self::disconnect_with_delay(switch.clone());
                                }
                            }
                        } else {
                            status_stack.update(Status::SqlErr(
                                format!("Could not read CSV content to string"))
                            );
                            Self::disconnect_with_delay(switch.clone());
                        }
                    } else {
                        status_stack.update(Status::SqlErr(
                            format!("Could not open file"))
                        );
                        Self::disconnect_with_delay(switch.clone());
                    }
                } else {
                    println!("Could not get mutable reference to tenv or recover file name");
                }
            } else {
                status_stack.update(Status::SqlErr(
                    format!("File should have any of the extensions: .csv|.db|.sqlite"))
                );
                Self::disconnect_with_delay(switch.clone());
            }
        } else {
            println!("Could not recover file name as string");
        }
    }

    fn generate_conn_str(
        entries : &[gtk::Entry; 4]
    ) -> Result<String,String> {
        let mut conn_info : HashMap<&str, String> = HashMap::new();
        let fields = ["host", "user", "password", "dbname"];
        for (entry, field) in entries.iter().zip(fields.iter()) {
            if let Some(s) = entry.get_text() {
                let value = s.as_str().to_owned();
                if !value.is_empty() {
                    if field == &" host" {
                        let spl_port : Vec<&str> = value.split(":").collect();
                        if spl_port.len() >= 2 {
                            conn_info.insert(
                                "host", spl_port[0].to_owned().clone()
                            );
                            conn_info.insert(
                                "port", spl_port[1].to_owned().clone());
                        } else {
                            conn_info.insert("host",
                                spl_port[0].to_owned().clone());
                        }
                    } else {
                        conn_info.insert(field, value);
                    }
                }
            }
        }

        let mut conn_str = "postgresql://".to_owned();
        if let Some(s) = conn_info.get("user") {
            conn_str += s;
        }
        if let Some(s) = conn_info.get("password") {
            conn_str = conn_str + ":" + &s;
        }
        if let Some(s) = conn_info.get("host") {
            if s == "localhost" || s == "127.0.0.1" {
                conn_str = conn_str + "@" + &s;
            } else {
                return Err(format!("Remote connections not allowed yet."));
            }
        } else {
            conn_str = conn_str + &"@localhost";
        }
        if let Some(s) = conn_info.get("port") {
            conn_str = conn_str + ":" + &s;
        } else {
            conn_str = conn_str + ":5432";
        }
        if let Some(s) = conn_info.get("dbname") {
            conn_str = conn_str + "/" + &s;
        }
        Ok(conn_str)
    }

}

    /*fn try_connect(entries : &[gtk::Entry; 4])
        -> Result<Connection,String>
    {
        let mut conn_str = String::new();
        let fields = [" host=", " user=", " password=", " dbname="];
        for (entry, field) in entries.iter().zip(fields.iter()) {
            if let Some(s) = entry.get_text() {
                let value = s.as_str();

                if !value.is_empty() {
                    conn_str += field;
                    if field == &" host=" {
                        let spl_port : Vec<&str> = value.split(":").collect();
                        if spl_port.len() >= 2 {
                            conn_str += spl_port[0];
                            conn_str += " port=";
                            conn_str += spl_port[1];
                        } else {
                            conn_str += spl_port[0];
                        }
                    } else {
                        conn_str += value;
                    }
                }
            }
        }
        if conn_str.chars().count() > 1 {
            conn_str = conn_str[1..conn_str.len()].to_string();
        }

        conn_str = "'".to_owned() + &conn_str + "'";

        println!("{}", conn_str);

        let tls_mode = TlsMode::None;
        match Connection::connect(conn_str, tls_mode) {
            Ok(c) => Ok(c),
            Err(e) => Err(e.to_string())
        }
    }*/

// Make switch sensitive again
// gtk::timeout_add(16, move || {
//    if !search_entry_c.is_sensitive() {

            
