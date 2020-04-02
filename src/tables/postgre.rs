use postgres::{self, Row, types::FromSql, types::ToSql };
use std::convert::{TryFrom, TryInto};
use rust_decimal::Decimal;
use super::column::*;
use super::nullable_column::*;
use std::fmt::Display;
use super::table::*;

pub fn nullable_from_rows<'a, T>(
    rows : &'a [postgres::row::Row],
    ix : usize
) -> Result<NullableColumn, String>
    where
        T : FromSql<'a> + ToSql + Sync,
        // Vec<T> : TryFrom<Column>,
        NullableColumn : From<Vec<Option<T>>>
{
    let mut opt_data = Vec::new();
    for r in rows.iter() {
        let opt_datum = r.try_get::<usize, Option<T>>(ix)
            .map_err(|e| { println!("{}", e); format!("{}", e) })?;
        opt_data.push(opt_datum);
    }
    Ok(NullableColumn::from(opt_data))
}

pub fn try_any_integer(rows : &[postgres::row::Row], ix : usize) -> Result<NullableColumn, String> {
    match nullable_from_rows::<i8>(rows, ix) {
        Ok(col) => Ok(col),
        Err(_) => match nullable_from_rows::<i16>(rows, ix) {
            Ok(col) => Ok(col),
            Err(_) => match nullable_from_rows::<i32>(rows, ix) {
                Ok(col) => Ok(col),
                Err(_) => match nullable_from_rows::<u32>(rows, ix) {
                    Ok(col) => Ok(col),
                    Err(_) => nullable_from_rows::<i64>(rows, ix)
                }
            }
        }
    }
}

pub fn try_any_float(rows : &[postgres::row::Row], ix : usize) -> Result<NullableColumn, String> {
    match nullable_from_rows::<i8>(rows, ix) {
        Ok(col) => Ok(col),
        Err(_) => match nullable_from_rows::<f32>(rows, ix) {
            Ok(col) => Ok(col),
            Err(_) => nullable_from_rows::<f64>(rows, ix)
        }
    }
}

pub fn build_table_from_postgre(rows : &[postgres::row::Row]) -> Result<Table, &'static str> {
    let names : Vec<String> = rows.get(0)
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect() )
        .ok_or("No rows available")?;
    if names.len() == 0 {
        return Err("No columns available");
    }
    let ncols = names.len();
    let mut null_cols : Vec<NullableColumn> = Vec::new();
    for i in 0..ncols {
        match nullable_from_rows::<bool>(rows, i) {
            Ok(col) => null_cols.push(col.into()),
            Err(_) => match try_any_integer(rows, i) {
                Ok(col) => null_cols.push(col.into()),
                Err(_) => match try_any_float(rows, i) {
                    Ok(col) => null_cols.push(col.into()),
                    Err(_) => match nullable_from_rows::<Decimal>(rows, i) {
                        Ok(col) => null_cols.push(col.into()),
                        Err(_) => match nullable_from_rows::<String>(rows, i) {
                            Ok(col) => null_cols.push(col.into()),
                            Err(_) => match nullable_from_rows::<Vec<u8>>(rows, i) {
                                Ok(col) => null_cols.push(col.into()),
                                Err(_) => {
                                    return Err("Could not parse column");
                                }
                            }
                        }
                    }
                }
           }
        }
    }
    let cols : Vec<Column> = null_cols.drain(0..names.len())
        .map(|nc| nc.to_column()).collect();
    Ok(Table::new(names, cols)?)
}

