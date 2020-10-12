use postgres::{self, Client, Row, tls};
use postgres::types::{ToSql, FromSql};
use sqlparser::ast::Statement;
use queries::tables::table::Table;
use queries::tables;

pub enum ExecResult {
    Query(Table),
    Statement
}

pub fn exec_templated<T>(
    conn : &mut Client,
    stmt : &Statement,
    data : &[&[T]]
) -> Result<ExecResult, String>
where
    T : postgres::types::ToSql + Sync
{
    if data.get(0).is_none() {
        return Ok(());
    }
    let ncols = data[0].len();
    let mut row_ref = Vec::with_capacity(ncols);
    for (row_ix, row) in data.iter().enumerate() {
        for i in 0..ncols {
            if let Some(val) = row.get(i) {
                let ref_val = val as &(dyn ToSql+Sync);
                row_ref.push(ref_val);
            } else {
                return Err(format!("Row {} has insufficient data entries", row_ix));
            }
        }
        // If query, execute a single time. If statement, execute many times.
        match stmt {
            Statement::Query(_) => {
                if data.len() > 1 {
                    return Err(format!("Only single-row templates allowed for queries"));
                }
                let ans = conn.query(&format!("{}", stmt), row_ref)
                    .map_err(|e| format!("{}", e))?;
                let tbl = tables::postgre::build_table_from_postgre(&ans[..])?;
                return Ok(ExecResult::Query(tbl));
            }
            _ => {
                conn.execute(&format!("{}", stmt), row_ref)
                    .map_err(|e| format!("{}", e))?;
            }
        }
        row_ref.clear();
    }
    Ok(ExecResult::Statement)
}

/*pub fn query_templated<T>(conn : &mut Client, template : &str, data : &[T]) -> Result<Vec<Row>, String>
where
    T : postgres::types::ToSql
{
    conn.query(template, data).map_err(|e| format!("{}", e))
}*/

pub fn connect(conn_str : &str) -> Result<Client, String> {
    Client::connect(conn_str, tls::NoTls{ }).map_err(|e| format!("{}", e) )
}

pub fn query(conn : &mut Client, query : &str) -> Result<Table, String> {
    let ans = conn.query(query, &[]).map_err(|e| format!("{}", e))?;
    let tbl = tables::postgre::build_table_from_postgre(&ans[..])?;
    Ok(tbl)
}

pub fn exec(conn : &mut Client, stmt : &str) -> Result<(), String> {
    conn.execute(stmt, &[]).map_err(|e| format!("{}", e) )?;
    Ok(())
}
