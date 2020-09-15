use super::column::*;
use postgres::types::{ToSql, FromSql};
use std::marker::Sync;
use std::convert::{TryFrom, TryInto};
use std::mem;

/// Represents an incomplete column of information, holding
/// the indices from which the valid column entries refer to,
/// and the total column size.
#[derive(Debug, Clone)]
pub struct NullableColumn {
    col : Column,
    null_ix : Vec<usize>,
    n : usize
}

impl<'a> NullableColumn {

    const NULL : &'a str = "NULL";

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn from_col(col : Column) -> Self {
        let n = col.ref_content().len();
        //let mut valid_ix = Vec::new();
        //valid_ix.extend((0..n).map(|i| i ));
        Self{ col, null_ix : Vec::new(), n }
    }

    pub fn display_content(&self) -> Vec<String> {
        if let Column::Nullable(_) = self.col {
            println!("Recursive nullable column identified");
            return Vec::new()
        }
        let valid_content = self.col.display_content();
        let mut content = Vec::new();
        let mut n_ix = 0;
        for i in 0..self.n {
            if n_ix < self.null_ix.len() && i == self.null_ix[n_ix] {
                content.push(String::from(Self::NULL));
                n_ix += 1;
            } else {
                // TODO thread 'main' panicked at 'index out of bounds: the len is 200 but
                // the index is 200', src/tables/nullable_column.rs:46:30
                content.push(valid_content[i - n_ix].clone());
            }
        }
        content
    }

    /// Tries to convert to a complete representation, or just return a nullable
    /// variant otherwise.
    pub fn to_column(self) -> Column {
        if let Column::Nullable(_) = self.col {
            println!("Recursive nullable column identified");
        }
        if self.null_ix.len() == 0 {
            self.col
        } else {
            Column::Nullable(Box::new(self))
        }
    }

    pub fn ref_content(&'a self) -> Vec<&'a (dyn ToSql + Sync)>
        where &'a str : FromSql<'a>
    {
        if let Column::Nullable(_) = self.col {
            println!("Recursive nullable column identified");
            return Vec::new()
        }
        let valid_refs = self.col.ref_content();
        let mut full_refs = Vec::new();
        let mut n_ix = 0;
        for i in 0..self.n {
            if n_ix < self.null_ix.len() && i == self.null_ix[n_ix] {
                full_refs.push(&Self::NULL as &'a (dyn ToSql + Sync));
                n_ix += 1;
            } else {
                full_refs.push(valid_refs[i - n_ix]);
            }
        }
        full_refs
    }

    pub fn truncate(&mut self, n : usize) {
        self.col.truncate(n);
    }

}

impl<T> From<Vec<Option<T>>> for NullableColumn
    where
        T : ToSql + Sync + Clone,
        Column : From<Vec<T>>
{

    fn from(mut opt_vals: Vec<Option<T>>) -> Self {
        let n = opt_vals.len();
        let mut null_ix : Vec<usize> = Vec::new();
        let mut data : Vec<T> = Vec::new();
        for (i, opt_value) in opt_vals.drain(0..n).enumerate() {
            if let Some(v) = opt_value {
                data.push(v);
            } else {
                null_ix.push(i);
            }
        }
        Self { col : data.into(), null_ix, n }
    }

}

impl<T> TryInto<Vec<Option<T>>> for NullableColumn
    where
        T : ToSql + Sync + Clone,
        Vec<T> : TryFrom<Column>
{
    type Error = &'static str;

    fn try_into(mut self) -> Result<Vec<Option<T>>, Self::Error> {
        let n = self.n;
        let mut null_ix = Vec::new();
        mem::swap(&mut null_ix, &mut self.null_ix);
        let mut valid_vals : Vec<T> = self.col.try_into()
            .map_err(|_| "Error performing conversion")?;
        let mut opt_cols : Vec<Option<T>> = Vec::new();
        let mut n_ix = 0;
        for i  in 0..n {
            if n_ix < self.null_ix.len() && i == self.null_ix[n_ix] {
                opt_cols.push(None);
                n_ix += 1;
            } else {
                opt_cols.push(Some(valid_vals.remove(0)));
            }
        }
        if valid_vals.len() > 0 {
            return Err("Data vector not cleared");
        }
        Ok(opt_cols)
    }
}
