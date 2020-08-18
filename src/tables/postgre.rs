use postgres::{self, types::FromSql, types::ToSql };
use rust_decimal::Decimal;
use super::column::*;
use super::nullable_column::*;
use super::table::*;
use postgres::types::Type;

pub fn col_as_opt_vec<'a, T>(
    rows : &'a [postgres::row::Row],
    ix : usize
) -> Result<Vec<Option<T>>, &'static str>
    where
        T : FromSql<'a> + ToSql + Sync,
{
    let mut opt_data = Vec::new();
    for r in rows.iter() {
        let opt_datum = r.try_get::<usize, Option<T>>(ix)
            .map_err(|e| { println!("{}", e); "Unable to parse column" })?;
        opt_data.push(opt_datum);
    }
    Ok(opt_data)
}

pub fn nullable_from_rows<'a, T>(
    rows : &'a [postgres::row::Row],
    ix : usize
) -> Result<NullableColumn, &'static str>
    where
        T : FromSql<'a> + ToSql + Sync,
        NullableColumn : From<Vec<Option<T>>>
{
    let opt_data = col_as_opt_vec::<T>(rows, ix)?;
    Ok(NullableColumn::from(opt_data))
}

pub fn as_nullable_text<'a, T>(
    rows : &'a [postgres::row::Row],
    ix : usize
) -> Result<NullableColumn, &'static str>
    where
        T : FromSql<'a> + ToSql + Sync + ToString,
        NullableColumn : From<Vec<Option<String>>>
{
    let opt_data = col_as_opt_vec::<T>(rows, ix)?;
    let str_data : Vec<Option<String>> = opt_data.iter()
        .map(|opt| opt.as_ref().map(|o| o.to_string()) ).collect();
    Ok(NullableColumn::from(str_data))
}

/*pub fn try_any_integer(rows : &[postgres::row::Row], ix : usize) -> Result<NullableColumn, String> {
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
}*/

pub fn build_table_from_postgre(rows : &[postgres::row::Row]) -> Result<Table, &'static str> {
    let names : Vec<String> = rows.get(0)
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect() )
        .ok_or("No rows available")?;
    let row1 = rows.iter().next().ok_or("No first row available")?;
    let cols = row1.columns();
    let col_types : Vec<_> = cols.iter().map(|c| c.type_()).collect();
    if names.len() == 0 {
        return Err("No columns available");
    }
    let ncols = names.len();
    let mut null_cols : Vec<NullableColumn> = Vec::new();
    for i in 0..ncols {
        let is_bool = col_types[i] == &Type::BOOL;
        let is_bytea = col_types[i] == &Type::BYTEA;
        let is_text = col_types[i] == &Type::TEXT || col_types[i] == &Type::VARCHAR;
        let is_double = col_types[i] == &Type::FLOAT8;
        let is_float = col_types[i] == &Type::FLOAT4;
        let is_int = col_types[i] == &Type::INT4;
        let is_long = col_types[i] == &Type::INT8;
        let is_smallint = col_types[i] == &Type::INT2;
        let is_timestamp = col_types[i] == &Type::TIMESTAMP;
        let is_date = col_types[i] == &Type::DATE;
        let is_time = col_types[i] == &Type::TIME;
        let is_numeric = col_types[i] == &Type::NUMERIC;
        if is_bool {
            null_cols.push(nullable_from_rows::<bool>(rows, i)?);
        } else {
            if is_bytea {
                null_cols.push(nullable_from_rows::<Vec<u8>>(rows, i)?);
            } else {
                if is_text {
                    null_cols.push(nullable_from_rows::<String>(rows, i)?);
                } else {
                    if is_double {
                        null_cols.push(nullable_from_rows::<f64>(rows, i)?);
                    } else {
                        if is_float {
                            null_cols.push(nullable_from_rows::<f32>(rows, i)?);
                        } else {
                            if is_int {
                                null_cols.push(nullable_from_rows::<i32>(rows, i)?);
                            } else {
                                if is_smallint {
                                    null_cols.push(nullable_from_rows::<i16>(rows, i)?);
                                } else {
                                    if is_long {
                                        null_cols.push(nullable_from_rows::<i64>(rows, i)?);
                                    } else {
                                        if is_timestamp {
                                            null_cols.push(as_nullable_text::<chrono::NaiveDateTime>(rows, i)?);
                                        } else {
                                            if is_date {
                                                null_cols.push(as_nullable_text::<chrono::NaiveDate>(rows, i)?);
                                            } else {
                                                if is_time {
                                                    null_cols.push(as_nullable_text::<chrono::NaiveTime>(rows, i)?);
                                                } else {
                                                    if is_numeric {
                                                        null_cols.push(nullable_from_rows::<Decimal>(rows, i)?);
                                                    } else {
                                                        let unable_to_parse : Vec<Option<String>> = rows.iter()
                                                            .map(|_| Some(String::from("(Unable to parse)")))
                                                            .collect();
                                                        null_cols.push(NullableColumn::from(unable_to_parse));
                                                    }
                                                }
                                            }
                                        }
                                    }
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


