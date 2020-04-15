use nalgebra::DVector;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::PathBuf;
use nalgebra::*;
use nalgebra::base::storage::Storage;
use super::csv::*;
use crate::tables::sql::*;
use chrono;
use super::source::*;
use super::stdin::*;
use std::path::Path;
use super::sql::*;
use super::table::*;

pub struct TableEnvironment {
    source : EnvironmentSource,
    listener : SqlListener,
    tables : Vec<Table>,
    last_update : Option<String>
}

impl TableEnvironment {

    pub fn new(src : EnvironmentSource) -> Self {
        Self{
            source : src,
            listener : SqlListener::launch(),
            tables : Vec::new(),
            last_update : None
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

    pub fn send_current_query(&mut self) -> Result<(), String> {
        //println!("{:?}", self.source);
        let query = match self.source {
            EnvironmentSource::PostgreSQL(ref db_pair) =>{
                Some(db_pair.1.clone())
            },
            EnvironmentSource::SQLite3(ref db_pair) => {
                Some(db_pair.1.clone())
            },
            _ => None
        };
        if let Some(q) = query {
            self.listener.send_command(q)
        } else {
            Err(format!("No query available to send."))
        }
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
            _ => { }
        }
    }

    pub fn prepare_and_send_query(&mut self, sql : String) -> Result<(), String> {
        //self.listener.send_command(sql.clone());
        self.prepare_query(sql);
        self.send_current_query()
    }

    /// Try to update the tables, potentially returning the first error
    /// message encountered by the database. Returns None if there
    /// is no update; Returns the Ok(result) if there is update, potentially
    /// carrying the first error the database encountered.
    pub fn maybe_update_from_query_results(
        &mut self
    ) -> Option<Result<(),String>> {
        let results = self.listener.maybe_get_result()?;
        self.tables.clear();
        for r in results {
            match r {
                QueryResult::Valid(tbl) => self.tables.push(tbl),
                QueryResult::Invalid(msg) => {
                    return Some(Err(msg));
                },
                QueryResult::Statement(_) => {
                    //return Some(Err(String::from("Last")))
                }
            }
        }
        self.last_update = Some(format!("{}", chrono::offset::Local::now()));
        Some(Ok(()))
    }

    pub fn result_last_statement(&self) -> Option<Result<String,String>> {
        let results = self.listener.maybe_get_result()?;
        if let Some(r) = results.last() {
            println!("{:?}", r);
            match r {
                QueryResult::Statement(s) => Some(Ok(s.clone())),
                QueryResult::Invalid(e) => Some(Err(e.clone())),
                QueryResult::Valid(_) => None
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
        }
        //println!("{:?}", src );
        match src.clone() {
            // Updates from a single CSV file. This implies only a single
            // table is available.
            EnvironmentSource::File(path, content) => {
                //println!("Received source at update_from_source: {}", content);
                self.tables.clear();
                let p = Path::new(&path);
                let p = p.file_stem().ok_or("Could not extract table name from path".to_string())?;
                let tbl_name = Some(p.to_str().ok_or("Could not convert table path to str".to_string())?.to_string());
                match Table::new_from_text(content.to_string()) {
                    Ok(tbl) => { self.tables.push(tbl)  },
                    Err(e) => { return Err(format!("Error: {}", e)); }
                }
            },
            EnvironmentSource::PostgreSQL((conn, q)) => {
                self.set_new_postgre_engine(conn)
                    .map_err(|e| { format!("{}", e) })?;
                if !q.is_empty() {
                    if let Err(e) = self.prepare_and_send_query(q) {
                        println!("{}", e);
                    }
                }
            },
            EnvironmentSource::SQLite3((conn, q)) => {
                self.set_new_sqlite3_engine(conn)
                    .map_err(|e|{ format!("{}", e) })?;
                if !q.is_empty() {
                    if let Err(e) = self.prepare_and_send_query(q) {
                        println!("{}", e);
                    }
                }
            },
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

    pub fn get_text_at_index(&self, idx : usize) -> Option<String> {
        if let Ok(tbl) = self.get_table_by_index(idx) {
            Some(tbl.to_string())
        } else {
            None
        }
    }

    /// Get informed columns, where indices are counted
    /// from the first column of the first table up to the
    /// last column of the last table.
    pub fn get_columns<'a>(&'a self, ixs : &[usize]) -> Columns<'a> {
        let mut cols = Columns::new();
        let mut base_ix : usize = 0;
        for tbl in self.tables.iter() {
            let ncols = tbl.shape().1;
            let curr_ixs : Vec<usize> = ixs.iter()
                .filter(|ix| **ix >= base_ix && **ix < base_ix + ncols)
                .map(|ix| ix - base_ix).collect();
            cols = cols.clone().take_and_extend(tbl.get_columns(&curr_ixs));
            base_ix += ncols;
        }
        cols
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
            tables.push(t.text_rows());
        }
        tables
        /*if let Some(t) = self.tables.iter().next() {
            t.as_rows()
        } else {
            Vec::new()
        }*/
    }

    pub fn all_tables(&self) -> Vec<&Table> {
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
        name : Option<String>,
        content : String
    ) -> Result<(), &'static str> {
        let t = Table::new_from_text(content)?;
        self.append_external_table(t)?;
        Ok(())
    }

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
                self.send_current_query();
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

