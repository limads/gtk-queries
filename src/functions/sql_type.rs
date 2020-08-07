use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display};
use quote::ToTokens;
use std::cmp::PartialEq;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlType {
    I32,
    I64,
    F32,
    F64,
    String
}

impl TryFrom<&str> for SqlType {

    type Error = ();

    fn try_from(s : &str) -> std::result::Result<Self,()> {
        match s {
            "i32" => Ok(Self::I32),
            "i64" => Ok(Self::I64),
            "f32" => Ok(Self::F32),
            "f64" => Ok(Self::F64),
            "String" => Ok(Self::String),
            _ => Err(())
        }
    }

}

fn parse_scalar_variadic(ty : &str) -> Option<(SqlType, bool)> {
    match SqlType::try_from(&ty[..]) {
        Ok(s) => Some((s, false)),
        _ => {
            if ty.starts_with("&[") {
                if let Ok(s) = SqlType::try_from(&ty[2..5]) {
                    Some((s, true))
                } else {
                    if let Ok(s) = SqlType::try_from(&ty[2..8]) {
                        Some((s, true))
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        }
    }
}

impl TryFrom<syn::Type> for SqlType {

    type Error = ();

    fn try_from(token : syn::Type) -> std::result::Result<Self,()> {
        SqlType::try_from(&type_to_string(&token)[..])
    }

}

impl fmt::Display for SqlType {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::String => "String"
        };
        write!(f, "{}", out)
    }

}

#[derive(Debug, Clone, Copy)]
pub enum SqlAggType {
    Simple(SqlType),
    Nested(SqlType),
    Owned(SqlType)
}

impl SqlAggType {

    pub fn inner(&self) -> SqlType {
        match self {
            SqlAggType::Simple(ty) => *ty,
            SqlAggType::Nested(ty) => *ty,
            SqlAggType::Owned(ty) => *ty
        }
    }
}

impl TryFrom<syn::Type> for SqlAggType {

    type Error = ();

    fn try_from(token : syn::Type) -> std::result::Result<Self,()> {
        let type_str = type_to_string(&token);
        SqlAggType::try_from(&type_str[..])
    }

}

impl TryFrom<&str> for SqlAggType {

    type Error = ();

    fn try_from(s : &str) -> std::result::Result<Self,()> {
        if s.starts_with("&[") {
            if s.starts_with("&[&[") {
                if let Ok(inner) = SqlType::try_from(&s[4..7]) {
                    Ok(SqlAggType::Nested(inner))
                } else {
                    Ok(SqlAggType::Nested(SqlType::try_from(&s[4..10])?))
                }
            } else {
                if let Ok(inner) = SqlType::try_from(&s[2..5]) {
                    Ok(SqlAggType::Simple(inner))
                } else {
                    Ok(SqlAggType::Simple(SqlType::try_from(&s[2..8])?))
                }
            }
        } else {
            if s.starts_with("Vec<") {
                if let Ok(inner) = SqlType::try_from(&s[4..7]) {
                    Ok(SqlAggType::Owned(inner))
                } else {
                    Ok(SqlAggType::Owned(SqlType::try_from(&s[4..10])?))
                }
            } else {
                Err(())
            }
        }
    }

}

impl fmt::Display for SqlAggType {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            //SqlAggType::Simple(s) => format!("{}", s),
            SqlAggType::Simple(s) => format!("&[{}]", s),
            SqlAggType::Nested(s) => format!("&[&[{}]]", s),
            SqlAggType::Owned(s) => format!("Vec<{}>", s),
        };
        write!(f, "{}", out)
    }

}

#[derive(Debug)]
struct SqlCall {
    name : String
}

fn type_to_string(ty : &syn::Type) -> String {
    format!("{}", ty.to_token_stream())
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[test]
fn test() {
    let t1 = SqlAggType::try_from("&[i32]").unwrap();
    let t2 = SqlAggType::try_from("&[i64]").unwrap();
    let t3 = SqlAggType::try_from("&[f64]").unwrap();
    let t4 = SqlAggType::try_from("&[String]").unwrap();
    let t5 = SqlAggType::try_from("Vec<i32>").unwrap();
    let t6 = SqlAggType::try_from("Vec<i64>").unwrap();
    let t7 = SqlAggType::try_from("Vec<f64>").unwrap();
    let t8 = SqlAggType::try_from("Vec<String>").unwrap();
    let t9 = SqlAggType::try_from("&[&[i32]]").unwrap();
    let t10 = SqlAggType::try_from("&[&[i64]]").unwrap();
    let t11 = SqlAggType::try_from("&[&[f64]]").unwrap();
    let t12 = SqlAggType::try_from("&[&[String]]").unwrap();
    println!("{:?}", &[t1, t2, t3, t4, t5, t6, t7, t8, t9, t10, t11, t12]);
}

