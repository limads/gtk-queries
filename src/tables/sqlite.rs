use rusqlite::{self, types::Value };
use super::column::*;
use super::nullable_column::*;
use super::table::*;
use rusqlite::types::FromSql;
use rusqlite::Row;
use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum SqliteColumn {
    I64(Vec<Option<i64>>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    Bytes(Vec<Option<Vec<u8>>>)
}

impl Display for SqliteColumn {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = match self {
            SqliteColumn::I64(_) => "Integer",
            SqliteColumn::F64(_) => "Real",
            SqliteColumn::Str(_) => "String",
            SqliteColumn::Bytes(_) => "Bytes",
        };
        write!(f, "{}", t)
    }
}

impl SqliteColumn {

    fn new(decl_type : &str) -> Result<Self, &'static str> {
        match decl_type {
            "integer" | "int" | "INTEGER" | "INT" => Ok(SqliteColumn::I64(Vec::new())),
            "real" | "REAL" => Ok(SqliteColumn::F64(Vec::new())),
            "text" | "TEXT" => Ok(SqliteColumn::Str(Vec::new())),
            "blob" | "BLOB" => Ok(SqliteColumn::Bytes(Vec::new())),
            _ => { println!(" Informed type: {} ", decl_type); Err("Invalid column type") }
        }
    }

    fn new_from_first_value(row : &Row, ix : usize) -> Result<Self, &'static str> {
        if let Ok(opt_value) = row.get::<usize, Option<i64>>(ix) {
            return Ok(SqliteColumn::I64(vec![opt_value]));
        } else {
            if let Ok(opt_value) = row.get::<usize, Option<f64>>(ix) {
                return Ok(SqliteColumn::F64(vec![opt_value]));
            } else {
                if let Ok(opt_value) = row.get::<usize, Option<String>>(ix) {
                    return Ok(SqliteColumn::Str(vec![opt_value]));
                } else {
                    if let Ok(opt_value) = row.get::<usize, Option<Vec<u8>>>(ix) {
                        return Ok(SqliteColumn::Bytes(vec![opt_value]));
                    } else {
                        Err("Could not parse value")
                    }
                }
            }
        }
    }

    fn append_from_row(&mut self, row : &Row, ix : usize) -> Result<(), &'static str> {
        if let Ok(opt_value) = row.get::<usize, Option<i64>>(ix) {
            if let SqliteColumn::I64(ref mut v) = self {
                v.push(opt_value);
                return Ok(());
            }
        } else {
             if let Ok(opt_value) = row.get::<usize, Option<f64>>(ix) {
                if let SqliteColumn::F64(ref mut v) = self {
                    v.push(opt_value);
                    return Ok(());
                }
             } else {
                if let Ok(opt_value) = row.get::<usize, Option<String>>(ix) {
                    if let SqliteColumn::Str(ref mut v) = self {
                        v.push(opt_value);
                        return Ok(());
                    }
                } else {
                    if let Ok(opt_value) = row.get::<usize, Option<Vec<u8>>>(ix) {
                        if let SqliteColumn::Bytes(ref mut v) = self {
                            v.push(opt_value);
                            return Ok(());
                        }
                    }
                }
             }
        }
        Err("Unable to parse value")
    }

    fn try_append(&mut self, value : Value) -> Result<(), &'static str> {
        match self {
            Self::I64(ref mut v) => {
                match value {
                    Value::Integer(i) => v.push(Some(i)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::F64(ref mut v) => {
                match value {
                    Value::Real(r) => v.push(Some(r)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::Str(ref mut v) => {
                match value {
                    Value::Text(t) => v.push(Some(t)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::Bytes(ref mut v) => {
                match value {
                    Value::Blob(b) => v.push(Some(b)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            }
        }
        Ok(())
    }

}

impl From<SqliteColumn> for NullableColumn
    where
        NullableColumn : From<Vec<Option<i64>>>,
        NullableColumn : From<Vec<Option<f64>>>,
        NullableColumn : From<Vec<Option<String>>>,
        NullableColumn : From<Vec<Option<Vec<u8>>>>,
{
    fn from(col: SqliteColumn) -> Self {
        match col {
            SqliteColumn::I64(v) => v.into(),
            SqliteColumn::F64(v) => v.into(),
            SqliteColumn::Str(v) => v.into(),
            SqliteColumn::Bytes(v) => v.into()
        }
    }
}

pub fn build_table_from_sqlite(mut rows : rusqlite::Rows) -> Result<Table, &'static str>
    where
        NullableColumn : From<Vec<Option<i64>>>,
        NullableColumn : From<Vec<Option<f64>>>,
        NullableColumn : From<Vec<Option<String>>>,
        NullableColumn : From<Vec<Option<Vec<u8>>>>,
{
    let cols = rows.columns().ok_or("No columns available")?;
    let col_names = rows.column_names().ok_or("No columns available")?;
    let empty_cols : Vec<Column> = cols.iter().map(|c| {

        // TODO value breaking here as type is returned with different names.
        let sq_c = SqliteColumn::new(c.decl_type().unwrap_or("blob")).unwrap();
        let nc : NullableColumn = sq_c.into();
        nc.to_column()
    }).collect();
    // println!("names: {:?}", col_names);
    // let col_types : Vec<Option<&str>> = cols.iter().map(|c| c.decl_type()).collect();
    let names : Vec<_> = col_names.iter().map(|c| c.to_string()).collect();
    if names.len() == 0 {
        return Err("No columns available");
    }
    let mut sqlite_cols : Vec<SqliteColumn> = Vec::new();
    /*for (i, ty) in col_types.iter().enumerate() {
        if let Some(t) = ty {
            sqlite_cols.push(SqliteColumn::new(t)?);
        } else {
            println!("Type unknown at column: {}", i);
        }
    }*/
    let mut curr_row = 0;
    while let Ok(row) = rows.next() {
        match row {
            Some(r) => {
                if curr_row == 0 {
                    for c_ix in 0..names.len() {
                        sqlite_cols.push(SqliteColumn::new_from_first_value(&r, c_ix)?);
                    }
                } else {
                    for (i, col) in sqlite_cols.iter_mut().enumerate() {
                        // let value = r.get::<usize, rusqlite::types::Value>(i)
                        //    .unwrap_or(rusqlite::types::Value::Null);
                        // TODO panicking here when using a sqlite subtraction.
                        // sqlite_cols[i].try_append(value)?;
                        col.append_from_row(r, i);
                    }
                }
                curr_row += 1;
            },
            None => { break; }
        }
    }
    if curr_row == 0 {
        Ok(Table::new(names, empty_cols)?)
    } else {
        let mut null_cols : Vec<NullableColumn> = sqlite_cols
            .drain(0..sqlite_cols.len())
            .map(|c| c.into() ).collect();
        if null_cols.len() == 0 {
            return Err("Too few columns");
        }
        let cols : Vec<Column> = null_cols.drain(0..null_cols.len())
            .map(|nc| nc.to_column()).collect();
        Ok(Table::new(names, cols)?)
    }
}

mod functions {

    use rusqlite::{self, ToSql};
    use rusqlite::functions::{Aggregate, Context};
    use std::panic::{RefUnwindSafe, UnwindSafe};

    pub struct ToSqlAgg<T,F>
    where
        T : ToSql,
        F : ToSql
    {
        data : T,

        init_func : Box<dyn Fn()->T>,

        /// This function can be read as a dynamic external symbol
        state_func : Box<dyn Fn(T)->T>,

        /// This function also can be read as a dynamic external symbol
        final_func : Box<dyn Fn(T)->F>
    }

    impl<T, F> Aggregate<T, F> for ToSqlAgg<T, F>
    where
        T : ToSql + RefUnwindSafe + UnwindSafe,
        F : ToSql + RefUnwindSafe + UnwindSafe
    {
        fn init(&self) -> T {
            unimplemented!()
        }

        fn step(&self, ctx : &mut Context, t : &mut T) ->rusqlite::Result<()> {
            unimplemented!()
        }

        fn finalize(&self, t : Option<T>) -> rusqlite::Result<F> {
            unimplemented!()
        }

    }

}


