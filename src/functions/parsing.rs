use syn::{File, Item, Type, ItemFn, ItemMod, ReturnType, FnArg, AttrStyle };
use syn::parse::Parse;
use quote::ToTokens;

pub struct Function {
    name : String,
    arg_types : Vec<String>,
    ret_type : String,
    doc : String
}

/// Returns (name, arg types, return type)
pub fn function_signature(f : ItemFn) -> Option<(String, Vec<String>, String)> {
    let name = f.sig.ident.to_token_stream().to_string();
    let inputs = f.sig.inputs.iter();
    let mut args = Vec::new();
    for input in inputs {
        match input {
            FnArg::Typed(typed) => {
                let ty : String = format!("{}", typed.ty.to_token_stream())
                    .chars().filter(|c| !c.is_whitespace()).collect();
                args.push(ty);
            },
            _ => {  }
        }
    }
    for attr in f.attrs {
        //match attr.style {
            //AttrStyle::Outer => {
                let ident = attr.path.get_ident().to_token_stream().to_string();
                let tokens = attr.tokens.to_string();
                println!("Ident: {}, tokens: {}", ident, tokens);
        //    },
        //    _ => { }
        //}
    }
    match f.sig.output {
        ReturnType::Type(_, bx_type) => {
            let ret : String = format!("{}", bx_type.to_token_stream())
                .chars().filter(|c| !c.is_whitespace()).collect();
            Some((name, args, ret))
        },
        _ => {
            None
        }
    }

}

/// Apply function_signature to a module and all its submodules recursively.
pub fn parse_mod_signature(
    item_mod : ItemMod
) -> Option<Vec<(String, Vec<String>, String)>> {
    let mut sigs = Vec::new();
    if let Some((_, items)) = item_mod.content {
        for item in items {
            match item {
                Item::Mod(item_mod) => {
                    sigs.extend(parse_mod_signature(item_mod)?);
                },
                Item::Fn(item_fn) => {
                    sigs.push(function_signature(item_fn)?);
                },
                _ => { }
            }
        }
    }
    Some(sigs)
}

/// Takes any item and retrieves the signature of all functions,
/// running over modules recursively.
pub fn parse_fn_or_mod(
    item : Item
) -> Option<Vec<(String, Vec<String>, String)>> {
    let mut sigs = Vec::new();
    match item {
        Item::Mod(item_mod) => {
            sigs.extend(parse_mod_signature(item_mod)?);
        },
        Item::Fn(item_fn) => {
            sigs.push(function_signature(item_fn)?);
        },
        _ => { }
    }
    Some(sigs)
}

/// Parse a full source file applying parse_fn_or_mod.
pub fn parse_nested_signatures(
    content : &str
) -> Option<Vec<(String, Vec<String>, String)>> {
    let t : File = syn::parse_str(content).ok()?;
    let mut sigs = Vec::new();
    for item in t.items {
        sigs.extend(parse_fn_or_mod(item)?);
    }
    Some(sigs)
}

/*
"use nlearn::table::*;\
            //#[no_mangle] \
            //pub extern fn summary(tbl : &Table, args : &[&str]) -> Result<Table,String> { \
            //    Ok(tbl.clone()) \
            //}\
            //pub mod my_mod {\
                /// Hello there
                pub fn sum2(tbl : &Table, args : &[&str])->Result<Table,String>{ \
                    Ok(tbl.clone()) \
                } \
            //}";
            */


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_test() -> Result<(),()> {
        let test = "/// Hello there\n
                pub fn sum2(tbl : &Table, args : &[&str])->Result<Table,String> { \
                    Ok(tbl.clone()) \
                }";
        println!("{:?}", parse_nested_signatures(test));
        Ok(())
    }

}


