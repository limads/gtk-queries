use tables::sql::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::*;
use gtk::prelude::*;
use postgres::{Connection, TlsMode};
use std::collections::HashMap;

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
    rc_conn : Rc<RefCell<Option<Connection>>>,
    pub queries : Vec<PostgreQuery>,
    pub valid_queries : Vec<usize>
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
        let conn : Option<Connection> =  Option::None;
        let rc_conn = Rc::new(RefCell::new(conn));

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

        let queries = Vec::new();
        let valid_queries = Vec::new();
        let popover = ConnPopover{
            btn,popover,entries,conn_label,
            conn_switch,rc_conn, queries, valid_queries };
        //popover.hook_signals();
        popover
    }

    pub fn hook_signals(&self) {
        let rc_conn = Rc::clone(&self.rc_conn);
        let entries = self.entries.clone();
        let label = self.conn_label.clone();
        self.conn_switch.connect_state_set(move |_switch, state| {
            if let Ok(mut c) = (*rc_conn).try_borrow_mut() {
                let is_connected = c.is_some();
                println!("{:?}", state);
                *c =  match (state, is_connected) {
                    (true, true) => None,
                    (false, true) => None,
                    (false, false) => None,
                    (true, false) =>
                        match ConnPopover::try_connect(&entries) {
                            Ok(ok_conn) => {
                                label.set_text("Connected");
                                Some(ok_conn)
                            },
                            Err(err_str) => {
                                label.set_text(&err_str);
                                //switch.set_active(false);
                                None
                            }
                        }
                }
            }
            Inhibit(false) //HERE
        });

        let popover = self.popover.clone();
        self.btn.connect_clicked(move |_| {
            popover.show();
        });
    }

    fn try_connect(entries : &[gtk::Entry; 4])
        -> Result<Connection,String> {

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
        let tls_mode = TlsMode::None;
        println!("{}", conn_str);
        match Connection::connect(conn_str, tls_mode) {
            Ok(c) => Ok(c),
            Err(e) => Err(e.to_string())
        }
    }

    /// Parse a SQL String to fill queries vector.
    /// Actual validity of the queries is not checked.
    /// Execution is only made at .run_all() method.
    pub fn parse_sql(&mut self, sql_text : String) {
        self.queries.clear();
        for q in sql_text.split(";") {
            let mut q_str = q.to_string();
            q_str += ";";
            println!("{}", q_str);
            self.queries.push( PostgreQuery::new(q_str));
            // if q.peek().next().is_none() {
            //    break;
            //}
        }
    }

    pub fn try_run_all(&mut self) {
        if let Ok(mut maybe_conn) = self.rc_conn.clone().try_borrow_mut() {
            //let maybe_conn = *maybe_conn;
            if let Some(mut c) = maybe_conn.as_mut() {
                for q in self.queries.iter_mut() {
                    q.run(&c);
                    if let Some(msg) = &q.err_msg {
                        println!("{}", msg);
                    }
                }
            }
        }
    }

    pub fn try_run_some(&mut self) {
        if let Ok(mut maybe_conn) = self.rc_conn.clone().try_borrow_mut() {

            if let Some(mut c) = maybe_conn.as_mut() {
                println!("valid queries : {:?}", self.valid_queries);
                for i in self.valid_queries.iter() {
                    if let Some(mut q) = self.queries.get_mut(*i) {
                        q.run(&c);
                        if let Some(msg) = &q.err_msg {
                            println!("{}", msg);
                        }
                    }
                }
            } else {
                println!("No connections available");
            }
        }
    }

    pub fn mark_all_valid(&mut self) {
        self.valid_queries = (0..self.queries.len()).collect();
    }

    pub fn get_valid_queries(&self) -> Vec<&PostgreQuery> {
        let mut queries : Vec<&PostgreQuery> = Vec::new();
        //for q in self.queries.iter() {
        //if q.err_msg.is_none() {
        //    valid_queries.push(&q);
        //}
        //}

        for i in self.valid_queries.iter() {
            if let Some(q) = self.queries.get(*i) {
                queries.push(q);
            }
        }

        queries
    }

     pub fn get_valid_queries_code(&self) -> Vec<String> {
        let queries = self.get_valid_queries();
        queries.iter().map(|q|{ q.query.clone() }).collect()
    }

    pub fn get_all_queries_code(&self) -> Vec<&str> {
        self.queries.iter().map(|q| { q.query.as_str() }).collect()
    }

    pub fn get_subset_valid_queries(
        &self,
        idx : Vec<usize>)
    -> Vec<&PostgreQuery> {
        let queries = self.get_valid_queries().clone();
        let mut keep_queries = Vec::new();
        for i in idx {
            keep_queries.push(queries[i]);
        }
        keep_queries
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

            
