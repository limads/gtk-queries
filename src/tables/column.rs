use postgres::types::ToSql;
use std::marker::Sync;
use rust_decimal::Decimal;
use super::nullable_column::*;
use num_traits::ToPrimitive;

// TODO create Array<Column> for N-D Postgre arrays, that carries a vector of Columns
// and a dimensionality metadata.

/// Densely packed column, where each variant is a vector of some
/// element that implements postgres::types::ToSql.
#[derive(Debug, Clone)]
pub enum Column {
    Bool(Vec<bool>),
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Numeric(Vec<Decimal>),
    Str(Vec<String>),
    Bytes(Vec<Vec<u8>>),
    Nullable(Box<NullableColumn>)
}

impl<'a> Column {

    pub fn new_empty<T>() -> Self
        where Column : From<Vec<T>>
    {
        let vec = Vec::<bool>::new();
        vec.into()
    }

    /*pub fn get_integer(&self, ix : usize) -> Option<i64> {
        match &self {
            Column::I64(v) => v.get(ix),
            Column::I8(v) => v.get(ix).map(|v| v as i64 ),
            Column::I16(v) => v.get(ix).map(|v| v as i64 ),
            Column::I32(v) => v.get(ix).map(|v| v as i64 ),
            Column::U32(v) => v.get(ix).map(|v| v as i64 ),
            _ => None
        }
    }

    pub fn get_real(&self, ix : usize) -> Option<f64> {
        match &self {
            Column::F32(v) => v.get(ix),
            Column::F64(v) => v.get(ix).map(|v| v as f64 ),
            _ => None
        }
    }

    pub fn get_text(&self, ix : usize) -> Option<String> {
        match &self {
            Column::Text(v) => v.get(ix).map(|v| v.clone() ),
            _ => None
        }
    }

    pub fn get_bytes(&self, ix : usize) -> Option<Vec<u8>> {
        match &self {
            Column::Bytes(b) => v.get(ix).map(|b| b.clone() ),
            _ => None
        }
    }*/

    /*pub fn try_slice_bool(&'a self) -> Option<&'a [bool]> {
        match self {
            Column::Bool(b) => Some(&b[..]),
            _ => None
        }
    }

    pub fn try_slice_i8(&'a self) -> Option<&'a [i8]> {
        match self {
            Column::I8(i) => Some(&i[..]),
            _ => None
        }
    }*/

    fn to_ref_dyn<'b, T>(v : &'b Vec<T>) -> Vec<&'b (dyn ToSql + Sync)>
    where T : ToSql + Sync
    {
        v.iter().map(|e| e as &'b (dyn ToSql + Sync)).collect()
    }

    pub fn len(&self) -> usize {
        match self {
            Column::Bool(v) => v.len(),
            Column::I8(v) => v.len(),
            Column::I16(v) => v.len(),
            Column::I32(v) => v.len(),
            Column::U32(v) => v.len(),
            Column::I64(v) => v.len(),
            Column::F32(v) => v.len(),
            Column::F64(v) => v.len(),
            Column::Numeric(v) => v.len(),
            Column::Str(v) => v.len(),
            Column::Bytes(v) => v.len(),
            Column::Nullable(col) => col.len()
        }
    }

    pub fn ref_content(&'a self) -> Vec<&(dyn ToSql + Sync)> {
        match self {
            Column::Bool(v) => Self::to_ref_dyn(v),
            Column::I8(v) => Self::to_ref_dyn(v),
            Column::I16(v) => Self::to_ref_dyn(v),
            Column::I32(v) => Self::to_ref_dyn(v),
            Column::U32(v) => Self::to_ref_dyn(v),
            Column::I64(v) => Self::to_ref_dyn(v),
            Column::F32(v) => Self::to_ref_dyn(v),
            Column::F64(v) => Self::to_ref_dyn(v),
            Column::Numeric(v) => Self::to_ref_dyn(v),
            Column::Str(v) => Self::to_ref_dyn(v),
            Column::Bytes(v) => Self::to_ref_dyn(v),
            Column::Nullable(col) => col.ref_content()
        }
    }

    fn display_with_precision(value : f64, prec : usize) -> String {
        match prec {
            1 => format!("{:.1}", value),
            2 => format!("{:.2}", value),
            3 => format!("{:.3}", value),
            4 => format!("{:.4}", value),
            5 => format!("{:.5}", value),
            6 => format!("{:.6}", value),
            7 => format!("{:.7}", value),
            8 => format!("{:.8}", value),
            _ => format!("{}", value)
        }
    }
    
    pub fn display_content(&'a self, prec : usize) -> Vec<String> {
        match self {
            Column::Bool(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::I8(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::I16(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::I32(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::U32(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::I64(v) => v.iter().map(|e| e.to_string() ).collect(),
            Column::F32(v) => v.iter().map(|e| Self::display_with_precision(*e as f64, prec) ).collect(),
            Column::F64(v) => v.iter().map(|e| Self::display_with_precision(*e as f64, prec) ).collect(),
            Column::Numeric(v) => { 
                v.iter().map(|e| {
                    if let Some(f) = e.round_dp(12).to_f64() {
                        Self::display_with_precision(f, prec)
                    } else {
                        e.to_string()
                    }
                }).collect()
            },
            Column::Str(v) => v.clone(),
            Column::Bytes(v) => v.iter().map(|_| format!("(Binary)") ).collect(),
            Column::Nullable(col) => col.display_content(prec)
        }
    }

    pub fn sqlite3_type(&self) -> String {
        match self {
            Column::I32(_) | Column::I64(_) => String::from("INT"),
            Column::F32(_) | Column::F64(_) => String::from("REAL"),
            Column::Bytes(_) => String::from("BLOB"),
            _ => String::from("TEXT"),
        }
    }

    pub fn truncate(&mut self, n : usize) {
        match self {
            Column::Bool(v) => v.truncate(n),
            Column::I8(v) => v.truncate(n),
            Column::I16(v) => v.truncate(n),
            Column::I32(v) => v.truncate(n),
            Column::U32(v) => v.truncate(n),
            Column::I64(v) => v.truncate(n),
            Column::F32(v) => v.truncate(n),
            Column::F64(v) => v.truncate(n),
            Column::Numeric(v) => v.truncate(n),
            Column::Str(v) => v.truncate(n),
            Column::Bytes(v) => v.truncate(n),
            Column::Nullable(col) => col.truncate(n)
        }
    }

}

pub mod from {

    use super::*;
    use std::convert::{ From, TryFrom };

    impl Into<()> for Column {
        fn into(self) -> () {
            unimplemented!()
        }
    }

    impl From<()> for Column {
        fn from(value: ()) -> Self {
            unimplemented!()
        }
    }

    impl From<Vec<()>> for Column {
        fn from(value: Vec<()>) -> Self {
            unimplemented!()
        }
    }

    impl From<Vec<bool>> for Column {
        fn from(value: Vec<bool>) -> Self {
            Self::Bool(value)
        }
    }

    impl From<Vec<i8>> for Column {

        fn from(value: Vec<i8>) -> Self {
            Self::I8(value)
        }
    }

    impl From<Vec<i16>> for Column {
        fn from(value: Vec<i16>) -> Self {
            Self::I16(value)
        }
    }

    impl From<Vec<i32>> for Column {
        fn from(value: Vec<i32>) -> Self {
            Self::I32(value)
        }
    }

    impl From<Vec<u32>> for Column {
        fn from(value: Vec<u32>) -> Self {
            Self::U32(value)
        }
    }

    impl From<Vec<i64>> for Column {
        fn from(value: Vec<i64>) -> Self {
            Self::I64(value)
        }
    }

    impl From<Vec<f32>> for Column {
        fn from(value: Vec<f32>) -> Self {
            Self::F32(value)
        }
    }

    impl From<Vec<f64>> for Column {
        fn from(value: Vec<f64>) -> Self {
            Self::F64(value)
        }
    }

    impl From<Vec<Decimal>> for Column {
        fn from(value: Vec<Decimal>) -> Self {
            Self::Numeric(value)
        }
    }

    impl From<Vec<String>> for Column {
        fn from(value: Vec<String>) -> Self {
            Self::Str(value)
        }
    }

    impl From<Vec<Vec<u8>>> for Column {
        fn from(value: Vec<Vec<u8>>) -> Self {
            Self::Bytes(value)
        }
    }

    /*impl<T> From<Vec<Vec<T>>> for Column
    where
        Column : From<Vec<T>>
    {

        //type Error = ();

        fn from(value: Vec<Vec<T>>) -> Self {
            let mut columns = Vec::new();
            for v in value {
                columns.push(v.into());
            }
            columns
        }

    }*/

    /*impl<T> From<Vec<Option<T>>> for Column
        where
            //Option<T> : ToSql + Sync + Clone,
            Column : From<Vec<T>>,
            NullableColumn : From<Vec<Option<T>>>
    {
        fn from(value: Vec<Option<T>>) -> Self {
            let null_col : NullableColumn = value.into();
            Self::Nullable(Box::new(null_col))
        }
    }*/

}

pub mod try_into {

    use std::convert::{ TryInto, TryFrom};
    use super::*;

    impl TryFrom<Column> for Vec<bool> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::Bool(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<i8> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::I8(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<i16> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::I16(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<i32> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::I32(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<u32> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::U32(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<i64> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::I64(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<f32> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::F32(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<f64> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::F64(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<Decimal> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::Numeric(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<String> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::Str(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }

    impl TryFrom<Column> for Vec<Vec<u8>> {

        type Error = &'static str;

        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::Bytes(v) => Ok(v),
                _ => Err("Invalid column type")
            }
        }

    }
    
    /*impl<T> TryFrom<Column> for Vec<Option<T>> 
    where
        Vec<T> : TryFrom<Column>,
        Vec<Option<T>> : TryFrom<NullableColumn>
    {
    
        type Error = &'static str;
        
        fn try_from(col : Column) -> Result<Self, Self::Error> {
            match col {
                Column::Nullable(c) => {
                    c.as_ref().try_into()
                },
                full => {
                    let vec_full  = Vec::<T>::try_from(full)?;
                    let wrapped_full : Vec<Option<T>> = vec_full.map(|e| Some(e) ).collect();
                    wrapped_full
                }
            }
        }
    }*/

}


