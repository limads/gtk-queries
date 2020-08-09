use std::fs::File;
use std::io::{Read, /*BufReader*/ };
use std::path::PathBuf;
use crate::tables::sql::*;
use super::source::*;
use std::path::Path;
use super::table::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crate::functions::loader::*;

#[cfg(feature="arrowext")]
use datafusion::execution::context::ExecutionContext;

#[cfg(feature="arrowext")]
use datafusion::execution::physical_plan::csv::CsvReadOptions;

#[derive(Clone, Debug)]
pub enum EnvironmentUpdate {

    Clear,

    /// Environment has completely new set of tables.
    /// One outer vector per table; Inner vectors hold column names.
    NewTables(Vec<Vec<String>>),

    /// Preserve last column sequence, just update the data.
    Refresh,

}

pub struct TableEnvironment {
    source : EnvironmentSource,
    listener : SqlListener,
    tables : Vec<Table>,

    /// Stores queries which returned successfully.
    queries : Vec<String>,

    last_update : Option<String>,
    history : Vec<EnvironmentUpdate>
}

impl TableEnvironment {

    pub fn new(src : EnvironmentSource, loader : Arc<Mutex<FunctionLoader>>) -> Self {
        Self{
            source : src,
            listener : SqlListener::launch(loader),
            tables : Vec::new(),
            last_update : None,
            queries : Vec::new(),
            history : vec![EnvironmentUpdate::Clear]
        }
    }

    pub fn set_new_postgre_engine(
        &mut self,
        conn_str : String
    ) -> Result<(), String> {
        match SqlEngine::try_new_postgre(conn_str) {
            Ok(engine) => { self.update_engine(engine)?; Ok(()) },
            Err(msg) => Err(msg)
        }
    }

    pub fn set_new_sqlite3_engine(
        &mut self,
        path : Option<PathBuf>
    ) -> Result<(), String> {
        match SqlEngine::try_new_sqlite3(path) {
            Ok(engine) => { self.update_engine(engine)?; Ok(()) },
            Err(msg) => Err(msg)
        }
    }

    pub fn disable_engine(&mut self) {
        if let Ok(mut engine) = self.listener.engine.lock() {
            *engine = SqlEngine::Inactive;
        } else {
            println!("Could not acquire lock over SQL engine");
        }
    }

    pub fn current_hist_index(&self) -> usize {
        self.history.len() - 1
    }

    pub fn full_history(&self) -> &[EnvironmentUpdate] {
        &self.history[..]
    }

    /// Check if there were any data changes since the last informed position.
    pub fn preserved_since(&self, pos : usize) -> bool {
        println!("Current history: {:?}", self.history);
        if pos == self.history.len() - 1 {
            true
        } else {
            let changed = self.history.iter()
                .skip(pos)
                .any(|h|
                    match h {
                        EnvironmentUpdate::Refresh => false,
                        _ => true
                    }
                );
            !changed
        }
    }

    fn update_engine(&mut self, engine : SqlEngine) -> Result<(), String> {
        if let Ok(mut old_engine) = self.listener.engine.lock() {
            *old_engine = engine;
            Ok(())
        } else {
            Err("Error acquiring lock over engine when updating it".into())
        }
    }

    pub fn get_engine_name(&self) -> String {
        if let Ok(engine) = self.listener.engine.lock() {
            match *engine {
                SqlEngine::Inactive => String::from("Inactive"),
                SqlEngine::PostgreSql{..} => String::from("PostgreSQL"),
                SqlEngine::Sqlite3{..} => String::from("SQLite3"),
                SqlEngine::Local{..} => String::from("Local"),

                #[cfg(feature="arrowext")]
                SqlEngine::Arrow{..} => String::from("Arrow"),
            }
        } else {
            String::from("Unavailable")
        }
    }

    /// Get engine active state. Consider it active in the event
    /// the lock could not be acquired.
    pub fn is_engine_active(&self) -> bool {
        if let Ok(engine) = self.listener.engine.lock() {
            match *engine {
                SqlEngine::Inactive => false,
                _ => true
            }
        } else {
            println!("Warning : Could not acquire lock over engine");
            true
        }
    }

    pub fn get_last_update_date(&self) -> String {
        match &self.last_update {
            Some(date) => date.clone(),
            None => String::from("Unknown")
        }
    }

    /*/// Execute a single function, appending the table to the end and returning a reference to it in case of
    /// success. Returns an error message from the user function otherwise.
    pub fn execute_func<'a>(&'a mut self, reg : Rc<FuncRegistry>, call : FunctionCall) -> Result<&'a Table, String> {
        if reg.has_func_name(&call.name[..]) {
            if let Some(f) = reg.retrieve_func(&call.name[..]) {
                let columns = self.get_columns(&call.source[..]);
                let ref_args : Vec<&str> = call.args.iter().map(|a| &a[..] ).collect();
                let ans = unsafe { f(columns, &ref_args[..]) };
                match ans {
                    Ok(res_tbl) => {
                        println!("{:?}", res_tbl);
                        let names = res_tbl.names();
                        self.tables.push(res_tbl);
                        self.history.push(EnvironmentUpdate::Function(call, names));
                        Ok(self.tables.last().unwrap())
                        /*let name = format!("({} x {})", nrows - 1, ncols);
                        tables_nb.add_page(
                            "network-server-symbolic",
                            Some(&name[..]),
                            None,
                            Some(t_rows),
                            fn_search.clone(),
                            pl_sidebar.clone(),
                            fn_popover.clone()
                        );*/
                        /*utils::set_tables(
                            &t_env,
                            &mut tbl_nb.clone(),
                            fn_search.clone(),
                            pl_sidebar.clone(),
                            fn_popover.clone()
                        );*/
                        //self.tables.last().to
                    },
                    Err(e) => {
                        println!("{}", e);
                        Err(e.to_string())
                    }
                }
            } else {
                Err("Error retrieving function".to_string())
            }
        } else {
            Err(format!("Function {} not in registry", call.name))
        }
        //Ok(())
    }*/

    /*/// Re-execute all function calls since the last NewTables history
    /// update, appending the resulting tables to the current environment.
    /// Returns a slice with the new generated tables.
    pub fn execute_saved_funcs<'a>(&'a mut self, reg : Rc<FuncRegistry>) -> Result<&'a [Table], String> {
        let recent_hist = self.history.iter().rev();
        let recent_hist : Vec<&EnvironmentUpdate> = recent_hist.take_while(|u| {
            match u {
                EnvironmentUpdate::NewTables(_) => false,
                _ => true
            }
        }).collect();
        let fns : Vec<FunctionCall> = recent_hist.iter().rev().filter_map(|update| {
            match update {
                EnvironmentUpdate::Function(call, _) => Some(call.clone()),
                _ => None
            }
        }).collect();
        println!("Last functions: {:?}", fns);
        let n_funcs = fns.len();
        for _ in 0..n_funcs {
            self.tables.remove(self.tables.len() - 1);
        }
        println!("Updated internal tables lenght before new call: {:?}", self.tables.len());
        for f in fns {
            self.execute_func(reg.clone(), f.clone())?;
        }
        println!("Internal tables length after new call: {:?}", self.tables.len());
        Ok(&self.tables[(self.tables.len() - n_funcs)..self.tables.len()])
    }*/

    pub fn send_current_query(&mut self, parse : bool) -> Result<(), String> {
        println!("Sending current query: {:?}", self.source);
        let query = match self.source {
            EnvironmentSource::PostgreSQL(ref db_pair) =>{
                Some(db_pair.1.clone())
            },
            EnvironmentSource::SQLite3(ref db_pair) => {
                Some(db_pair.1.clone())
            },

            #[cfg(feature="arrowext")]
            EnvironmentSource::Arrow(ref q) => {
                Some(q.clone())
            },

            _ => None
        };
        if let Some(q) = query {
            if q.chars().all(|c| c.is_whitespace() ) {
                return Err(String::from("Empty query sequence"));
            }
            self.listener.send_command(q, parse)
        } else {
            Err(format!("No query available to send."))
        }
    }

    pub fn create_csv_table(&mut self, path : PathBuf, name : &str) -> Result<(), String> {

        // Case DataFusion
        match self.listener.engine.lock() {
            Ok(mut engine) => {
                match *engine {
                    #[cfg(feature="arrowext")]
                    SqlEngine::Arrow{ ref mut ctx } => {
                        ctx.register_csv(
                            name,
                            path.to_str().unwrap(),
                            CsvReadOptions::new(),
                        ).map_err(|e| format!("{}", e) )?;
                        return Ok(());
                    },
                    _ => { }
                }
            },
            Err(e) => { return Err(format!("{}", e)); },
        }

        // Case Sqlite3
        let sql = format!("create virtual table temp.{} using \
            csv(filename='{}', header='YES');", name, path.to_str().unwrap());
        self.prepare_and_send_query(sql, false)
    }

    pub fn clear_queries(&mut self) {
        let no_query = String::new();
        self.prepare_query(no_query);
    }

    pub fn prepare_query(&mut self, sql : String) {
        match self.source {
            EnvironmentSource::PostgreSQL((_, ref mut q)) => {
                *q = sql;
            },
            EnvironmentSource::SQLite3((_, ref mut q)) => {
                *q = sql;
            },

            #[cfg(feature="arrowext")]
            EnvironmentSource::Arrow(ref mut q) => {
                *q = sql;
            },

            _ => { }
        }
    }

    pub fn prepare_and_send_query(&mut self, sql : String, parse : bool) -> Result<(), String> {
        //self.listener.send_command(sql.clone());
        self.prepare_query(sql);
        self.send_current_query(parse)
    }

    /// Searches update history retroactively, returning the last
    /// original table update column names, if any exist, and
    /// no clear events are present between this last table set
    /// and the end of the history.
    fn last_table_columns(&self) -> Option<Vec<Vec<String>>> {
        for update in self.history.iter().rev() {
            match update {
                EnvironmentUpdate::NewTables(ref tbls) => {
                    return Some(tbls.clone());
                },
                EnvironmentUpdate::Clear => {
                    return None;
                }
                _ => { }
            }
        }
        None
    }

    /// Try to update the tables, potentially returning the first error
    /// message encountered by the database. Returns None if there
    /// is no update; Returns the Ok(result) if there is update, potentially
    /// carrying the first error the database encountered. If the update is valid,
    /// return the update event that happened (Refresh or NewTables).
    pub fn maybe_update_from_query_results(&mut self) -> Option<Result<EnvironmentUpdate,String>> {
        let results = self.listener.maybe_get_result()?;
        println!("Query results: {:?}", results);
        self.tables.clear();
        self.queries.clear();
        if results.len() == 0 {
            self.history.push(EnvironmentUpdate::Clear);
            return Some(Ok(EnvironmentUpdate::Clear));
        }
        let mut new_cols : Vec<Vec<String>> = Vec::new();
        let mut opt_err = None;
        let mut any_valid = false;
        for r in results {
            match r {
                QueryResult::Valid(query, tbl) => {
                    new_cols.push(tbl.names());
                    self.tables.push(tbl);
                    self.queries.push(query);
                    any_valid = true;
                },
                QueryResult::Invalid(msg) => {
                    self.tables.clear();
                    self.history.push(EnvironmentUpdate::Clear);
                    opt_err = Some(msg.clone());
                },
                QueryResult::Statement(_) => {
                    self.tables.clear();
                    self.history.push(EnvironmentUpdate::Clear);
                }
            }
        }
        if let Some(msg) = opt_err {
            self.history.push(EnvironmentUpdate::Clear);
            Some(Err(msg))
        } else {
            if any_valid {
                let last_update = if let Some(last_cols) = self.last_table_columns() {
                    if last_cols.len() == new_cols.len() {
                        let all_equal = last_cols.iter().flatten()
                            .zip(new_cols.iter().flatten())
                            .all(|(last, new)| last == new );
                        if all_equal {
                            EnvironmentUpdate::Refresh
                        } else {
                            EnvironmentUpdate::NewTables(new_cols)
                        }
                    } else {
                        EnvironmentUpdate::NewTables(new_cols)
                    }
                } else {
                    EnvironmentUpdate::NewTables(new_cols)
                };
                self.history.push(last_update.clone());
                println!("History: {:?}", self.history);
                Some(Ok(last_update))
            } else {
                Some(Ok(EnvironmentUpdate::Clear))
            }
        }
    }

    pub fn result_last_statement(&self) -> Option<Result<String,String>> {
        let results = self.listener.maybe_get_result()?;
        if let Some(r) = results.last() {
            println!("Last statement: {:?}", r);
            match r {
                QueryResult::Statement(s) => Some(Ok(s.clone())),
                QueryResult::Invalid(e) => Some(Err(e.clone())),
                QueryResult::Valid(_, _) => None
            }
        } else {
            None
        }
    }

    /// Try to update the table from a source such as a SQL connection string
    /// or a file path.
    pub fn update_source(
        &mut self,
        src : EnvironmentSource,
        clear : bool
    ) -> Result<(),String> {
        if clear {
            self.tables.clear();
            self.history.push(EnvironmentUpdate::Clear);
        }
        //println!("{:?}", src );
        match src.clone() {
            // Updates from a single CSV file. This implies only a single
            // table is available.
            EnvironmentSource::File(path, content) => {
                //println!("Received source at update_from_source: {}", content);
                self.tables.clear();
                let p = Path::new(&path);
                let _p = p.file_stem().ok_or("Could not extract table name from path".to_string())?;
                //let _tbl_name = Some(p.to_str().ok_or("Could not convert table path to str".to_string())?.to_string());
                match Table::new_from_text(content.to_string()) {
                    Ok(tbl) => { self.tables.push(tbl)  },
                    Err(e) => { return Err(format!("Error: {}", e)); }
                }
            },
            EnvironmentSource::PostgreSQL((conn, q)) => {
                self.set_new_postgre_engine(conn)
                    .map_err(|e| { format!("{}", e) })?;
                if !q.is_empty() {
                    if let Err(e) = self.prepare_and_send_query(q, true) {
                        println!("{}", e);
                    }
                }
            },
            EnvironmentSource::SQLite3((conn, q)) => {
                self.set_new_sqlite3_engine(conn)
                    .map_err(|e|{ format!("{}", e) })?;
                if !q.is_empty() {
                    if let Err(e) = self.prepare_and_send_query(q, true) {
                        println!("{}", e);
                    }
                }
            },

            #[cfg(feature="arrowext")]
            EnvironmentSource::Arrow(_) => {
                let ctx = ExecutionContext::new();
                self.update_engine(SqlEngine::Arrow{ ctx })?;
            }

            _ => { println!("Invalid_source"); }
        }
        self.source = src;
        Ok(())
    }

    pub fn convert_source_to_in_memory_sqlite(&mut self) {
        match &self.source {
            EnvironmentSource::SQLite3(_) => {
                println!("Source is already SQLite3");
            },
            EnvironmentSource::PostgreSQL(_) => {
                println!("Invalid source update: PostgreSQL->SQLite3");
            },
            _ => {
                if self.tables.len() == 0 {
                    println!("No tables on environment for conversion to in-memory SQLite3");
                    return;
                }
                let new_src = EnvironmentSource::SQLite3((None,"".into()));
                let tables = self.tables.clone();
                self.clear();
                match self.update_source(new_src, true) {
                    Ok(_) => {
                        if let Ok(mut engine) = self.listener.engine.lock() {
                            for t in tables {
                                engine.insert_external_table(&t);
                            }
                        } else {
                            println!("Unable to acquire lock over SQL listener to insert tables");
                        }
                    },
                    Err(e) => println!("{}", e)
                }
            }
        }
    }

    pub fn last_inserted_table(&self) -> Option<Table> {
        self.tables.last().map(|t| t.clone())
    }

    fn get_table_by_index(&self, idx : usize) -> Result<&Table,&'static str> {
        match self.tables.get(idx) {
            Some(t) => Ok(t),
            None => Err("No table at informed index")
        }
    }

    /*fn get_column_at_index<'a>(&'a self, tbl_ix : usize, col_ix : usize) -> Result<&'a Column, &'static str> {
        let tbl = self.get_table_by_index(tbl_ix)?;
        tbl.get_column(col_ix).ok_or("Invalid column index")
    }*/

    pub fn get_text_at_index(&self, idx : usize) -> Option<String> {
        if let Ok(tbl) = self.get_table_by_index(idx) {
            Some(tbl.to_string())
        } else {
            None
        }
    }

    pub fn global_to_tbl_ix(&self, global_ixs : &[usize]) -> Option<(usize, Vec<usize>)> {
        println!("Received global indices: {:?}", global_ixs);
        for i in 0..self.tables.len() {
            if let Some((cols,_,_)) = self.get_columns(global_ixs) {
                println!("Local indices wrt table: {:?}", i);
                println!("Local indices: {:?}", cols.indices().iter().cloned().collect::<Vec<_>>());
                return Some((i, cols.indices().iter().cloned().collect()))
            }
        }
        println!("No local indices found");
        None
    }

    /// Get informed columns, where indices are counted
    /// from the first column of the first table up to the
    /// last column of the last table. Columns must be part of the same
    /// table. Return the query for the given table at the last position.
    pub fn get_columns<'a>(&'a self, global_ixs : &[usize]) -> Option<(Columns<'a>, usize, String)> {
        let mut base_ix : usize = 0;
        for (i, tbl) in self.tables.iter().enumerate() {
            let ncols = tbl.shape().1;
            let curr_ixs : Vec<usize> = global_ixs.iter()
                .filter(|ix| **ix >= base_ix && **ix < base_ix + ncols)
                .map(|ix| ix - base_ix).collect();
            if curr_ixs.len() > 0 {
                let mut cols = Columns::new();
                cols = cols.clone().take_and_extend(tbl.get_columns(&curr_ixs));
                let query = self.get_queries().get(i).cloned().unwrap();
                return Some((cols, i, query));
            }
            base_ix += ncols;
        }
        None
    }

    /// Full column names vector.
    pub fn get_column_names(&self, global_ixs : &[usize]) -> Option<(Vec<String>, usize, String)> {
        let (cols, tbl_ix, query) = self.get_columns(global_ixs)?;
        let names : Vec<String> = cols.names().iter()
            .map(|name| name.to_string() )
            .collect();
        Some((names, tbl_ix, query))
    }

    /// One query per table in the full environment.
    pub fn get_queries(&self) -> &[String] {
        &self.queries[..]
    }

    pub fn set_table_at_index(
        &mut self,
        content : String,
        index : usize
    ) -> Result<(), &'static str> {
        if let Ok(new_t) = Table::new_from_text(content) {
            if let Some(t) = self.tables.get_mut(index) {
                *t = new_t;
                return Ok(())
            } else {
                Err("Invalid index")
            }
        } else {
            Err("Could not parse content")
        }
    }

    pub fn all_tables_as_rows(&self) -> Vec<Vec<Vec<String>>> {
        let mut tables = Vec::new();
        for t in self.tables.iter() {
            tables.push(t.clone().truncate(200).text_rows());
        }
        tables
        /*if let Some(t) = self.tables.iter().next() {
            t.as_rows()
        } else {
            Vec::new()
        }*/
    }

    pub fn all_tables<'a>(&'a self) -> Vec<&'a Table> {
        self.tables.iter().map(|t| t).collect()
    }

    pub fn all_tables_as_csv(&self) -> Vec<String> {
        let mut tbls_csv = Vec::new();
        for t in &self.tables {
            tbls_csv.push(t.to_string());
        }
        tbls_csv
    }

    pub fn append_table_from_text(
        &mut self,
        _name : Option<String>,
        content : String
    ) -> Result<(), &'static str> {
        let t = Table::new_from_text(content)?;
        self.append_external_table(t)?;
        Ok(())
    }

    // TODO receive function name and arguments
    // to save it to history.
    pub fn append_external_table(
        &mut self,
        tbl : Table
    ) -> Result<(), &'static str> {
        self.tables.push(tbl);
        Ok(())
    }

    pub fn update_from_current_source(&mut self) {
        self.tables.clear();
        match self.source {
            EnvironmentSource::Stream(ref s) => {
                if let Some(c) = s.get_last_content() {
                    self.append_table_from_text(Some("A".into()), c.clone())
                        .unwrap_or_else(|e| println!("{:?}", e));
                }
            },
            EnvironmentSource::File(ref path, ref mut content) => {
                if let Ok(mut f) = File::open(path) {
                    let mut new_content = String::new();
                    let _ = f.read_to_string(&mut new_content)
                        .unwrap_or_else(|e|{ println!("{:?}", e); 0} );
                    *content = new_content;
                } else {
                    println!("Could not re-open file");
                }
            },
            EnvironmentSource::SQLite3(_) | EnvironmentSource::PostgreSQL(_) => {
                self.send_current_query(true).map_err(|e| println!("{}", e) ).ok();
            },

            #[cfg(feature="arrowext")]
            EnvironmentSource::Arrow(_) => {
                self.send_current_query(true).map_err(|e| println!("{}", e) ).ok();
            },
            _ => { }
        }
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }

    // Pass this to environment source
    /*pub fn table_names_as_hash(&self) -> Option<HashMap<String, Vec<(String, String)>>> {
        let mut names = HashMap::new();
        match &self.source {
            EnvironmentSource::SQLite3(_) => {
                if let Ok(mut engine) = self.listener.engine.lock() {
                    if let Some(objs) = engine.get_table_names() {
                        for obj in objs {
                            names.insert(obj.name().into(), obj.fields().unwrap());
                        }
                    } else {
                        println!("Could not get table names from engine");
                        return None;
                    }
                } else {
                    println!("Unable to get mutable reference to engine");
                    return None;
                }
                Some(names)
            },
            _ => {
                println!("Table environment is not Sqlite3 and data could not be fetch");
                None
            }
        }
    }*/

    pub fn try_backup(&self, path : PathBuf) {
        if let Ok(engine) = self.listener.engine.lock() {
            engine.backup_if_sqlite(path);
        } else {
            println!("Unable to retrieve lock over SQL listener");
        }
    }

    pub fn last_commands(&self) -> Vec<String> {
        if let Ok(cmds) = self.listener.last_cmd.lock() {
            cmds.clone()
        } else {
            println!("Unable to acquire lock over last commands");
            Vec::new()
        }
    }

}

