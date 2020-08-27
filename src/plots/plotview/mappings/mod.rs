use libxml::tree::node::Node;
use cairo::Context;
use super::context_mapper::ContextMapper;
use std::collections::HashMap;
use super::utils;
use super::text::{FontData, draw_label};

pub mod area;

pub mod bar;

pub mod line;

pub mod scatter;

pub mod surface;

pub mod text;

pub enum MappingType {
    Line,
    Scatter,
    Bar,
    Area,
    Surface,
    Text
}

impl MappingType {

    pub fn from_str(name : &str) -> Option<Self> {
        match name {
            "line" => Some(MappingType::Line),
            "scatter" => Some(MappingType::Scatter),
            "bar" => Some(MappingType::Bar),
            "area" => Some(MappingType::Area),
            "surface" => Some(MappingType::Surface),
            "text" => Some(MappingType::Text),
            _ => None
        }
    }

    /// Returns a default property map for this mapping type. This is the major
    /// reference for the validity of any given plot property. This function
    /// deals with non-data properties.
    pub fn default_hash(&self) -> HashMap<String, String> {
        let mut hash = HashMap::new();
        hash.insert(String::from("color"), String::from("#000000"));
        hash.insert(String::from("x"), String::from("None"));
        hash.insert(String::from("y"), String::from("None"));
        hash.insert(String::from("source"), String::from("None"));
        match self {
            MappingType::Line => {
                hash.insert(String::from("width"), String::from("1"));
                hash.insert(String::from("dash"), String::from("1"));
            },
            MappingType::Scatter => {
                hash.insert(String::from("radius"), String::from("1"));
            },
            MappingType::Bar => {
                hash.insert(String::from("center_anchor"), String::from("false"));
                hash.insert(String::from("horizontal"), String::from("false"));
                hash.insert(String::from("width"), String::from("None"));
                hash.insert(String::from("height"), String::from("None"));
                hash.insert(String::from("bar_width"), String::from("1"));
                hash.insert(String::from("origin_x"), String::from("0"));
                hash.insert(String::from("origin_y"), String::from("0"));
                hash.insert(String::from("bar_spacing"), String::from("1"));
            },
            MappingType::Area => {
                hash.insert(String::from("ymax"), String::from("None"));
                hash.insert(String::from("opacity"), String::from("1.0"));
            },
            MappingType::Surface => {
                hash.insert(String::from("z"), String::from("None"));
                hash.insert(String::from("final_color"), String::from("#ffffff"));
                hash.insert(String::from("z_min"), String::from("0.0"));
                hash.insert(String::from("z_max"), String::from("1.0"));
                hash.insert(String::from("opacity"), String::from("1.0"));
            },
            MappingType::Text => {
                hash.insert(String::from("font"), String::from("Monospace Regular 12"));
                hash.insert(String::from("text"), String::from("None"));
            }
        }
        hash
    }
}

/// Default trait for updating mappings.
/// Mappings are always instantiated from
/// the concrete instances new(.) call, which
/// receive a XML definition. To aid in creating
/// this definition, the MappingType::default_hash function
/// can be used.
pub trait Mapping {

    // Mapping-specific impl.
    fn draw(&self, mapper : &ContextMapper, ctx : &Context);// { }

    // Mapping-specific impl.
    fn update_data(&mut self, values : Vec<Vec<f64>>); //{ }

    fn update_extra_data(&mut self, values : Vec<Vec<String>>);

    fn update_layout(&mut self, node : &Node);

    fn properties(&self) -> HashMap<String, String>;

    fn mapping_type(&self) -> String;

    fn get_col_name(&self, col : &str) -> String;

    fn get_ordered_col_names(&self) -> Vec<(String, String)>;

    fn get_hash_col_names(&self) -> HashMap<String, String>;

    fn set_col_name(&mut self, col : &str, name : &str);

    fn set_col_names(&mut self, cols : Vec<String>) -> Result<(), &'static str>;

    fn set_source(&mut self, source : String);

    fn get_source(&self) -> String;

}


