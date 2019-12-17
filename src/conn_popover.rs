use tables::sql::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::*;
use gtk::prelude::*;
use postgres::{Connection, TlsMode};
use std::collections::HashMap;
use tables::TableEnvironment;
use tables::sql::{SqlListener};

// Notice the lifetime of the popover (signaler)
// and the connection (to-be-mutated) are both
// tied to a parent struct.
pub struct ConnPopover {
    btn : gtk::Button,
    popover : gtk::Popover,
    //host_entry : gtk::Entry,
    //user_entry : gtk::Entry,
    //password_entry : gtk::Entry,
    entries : [gtk::Entry; 4],
    //db_entry : gtk::Entry,
    conn_label : gtk::Label,
    // conn_switch : &'a mut gtk::Switch,
    // conn : &'a mut Option<Connection>,
    //fn_swtich_conn : boxed::Box<dyn Fn(&'a Switch)>
    conn_switch : Switch,
    // rc_conn : Rc<RefCell<Option<Connection>>>,
    // pub queries : Vec<PostgreQuery>,
    // pub valid_queries : Vec<usize>
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
        btn : gtk::Button, path : &str)
        //conn : &'a mut Option<Connection>,
        /*conn_switch : &'a mut gtk::Switch)*/ -> ConnPopover {
        let builder = Builder::new_from_file(path);
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
        let conn_label : Label =
            builder.get_object("conn_status_label").unwrap();
        // let conn : Option<Connection> =  Option::None;
        // let rc_conn = Rc::new(RefCell::new(conn));

        /*{
            let conn = Rc::clone(&conn);
            conn_switch.connect_activate(move |switch| {
                *conn = None;
            //db_entry
            });
        }*/
        // Ownership will be passed to struct.
        //let conn_ref = & mut conn;
        // let conn_fn = |cs : &'a gtk::Switch| {
        //    let active = conn_switch.get_active();
        //    let is_connected = conn.is_some();
        //};
        // let fn_swtich_conn = boxed::Box::new(conn_fn);
        //let queries = Vec::new();
        //let valid_queries = Vec::new();
        let popover = ConnPopover{btn,popover,entries,conn_label,conn_switch };
        //popover.hook_signals();
        popover
    }

    pub fn hook_signals(&self, table_env : Rc<RefCell<TableEnvironment>>) {
        let entries = self.entries.clone();
        let label = self.conn_label.clone();
        self.conn_switch.connect_state_set(move |_switch, state| {
            if let Ok(mut t_env) = table_env.try_borrow_mut() {
                match (state, t_env.is_engine_active()) {
                    (false, true) => {
                        t_env.disable_engine();
                    },
                    (true, false) => {
                        let msg = match ConnPopover::generate_conn_str(&entries) {
                            Ok(conn_str) => {
                                match t_env.set_new_postgre_engine(conn_str) {
                                    Ok(_) => String::from("Connected"),
                                    Err(e) => format!("{}", e)
                                }
                            },
                            Err(err_str) => {
                                err_str.into()
                            }
                        };
                        label.set_text(&msg[..]);
                    },
                    _ => { }
                }
            } else {
                println!("Could not acquire lock over table environment");
            }
            Inhibit(false) //HERE
        });

        let popover = self.popover.clone();
        self.btn.connect_clicked(move |_| {
            popover.show();
        });
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
                                "host", spl_port[0].to_owned().clone());
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
            conn_str = conn_str + "@" + &s;
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

            
