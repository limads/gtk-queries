use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtkplotview::plot_view::{PlotView, UpdateContent};
//use gtkplotview::PlotArea;
use std::io::{Read, BufReader};
use std::path::PathBuf;
//use crate::data_source::TableDataSource;
use gtkplotview::PlotArea;
use crate::plots::layout_aux::*;
use crate::tables::{source::EnvironmentSource, environment::TableEnvironment};
use std::collections::HashMap;
use gdk::RGBA;
use gtk::prelude::*;
use crate::table_notebook::*;
use crate::tables::table::*;

/// MappingMenu is the structure common across all menus
/// used to manipulate the mappings directly (line, trace, scatter).
#[derive(Clone, Debug)]
pub struct MappingMenu {
    pub mapping_name : String,
    pub mapping_type : String,
    pub mapping_box : Box,
    //pub combos : Vec<ComboBoxText>,
    pub design_widgets : HashMap<String, Widget>
}

// If all combo boxes have valid textual entries, return Some<Vec>.
// Return None otherwise.
impl MappingMenu {

    /*pub fn get_selected_cols(&self) -> Option<Vec<String>> {
        let mut cols = Vec::new();
        //println!("{:?}", self.combos);
        for combo in self.combos.clone() {
            if let Some(txt) = combo.get_active_text() {
                cols.push(txt.as_str().to_owned());
            } else {
                return None;
            }
        }
        for c in cols.iter() {
            if c.len() == 0 {
                return None;
            }
        }
        Some(cols)
    }*/

    /*pub fn update_available_cols(
        &mut self,
        cols : Vec<String>,
        pl_view : &PlotView
    ) {
        let valid_cols = match &self.mapping_type[..] {
            "line" => vec!["x", "y"],
            "scatter" => vec!["x", "y"],
            "bar" => vec!["x", "y", "width", "height"],
            "area" => vec!["x", "y", "ymax"],
            "surface" => vec!["x", "y", "z"],
            "text" => vec!["x", "y", "text"],
            _ => vec![]
        };
        for (i, combo) in self.combos.iter().enumerate() {
            combo.remove_all();
            for s in cols.iter() {
                combo.append(Some(s), s); //Here! or append_text
                if i < valid_cols.len() {
                    let maybe_col = pl_view.plot_area.get_mapping_column(
                        &self.mapping_name[..],
                        valid_cols[i]
                    );
                    if let Some(col) = maybe_col {
                        if &col == s {
                            if !combo.set_active_id(Some(s)) {
                                println!("Problem setting active column (unrecognized id)");
                            }
                        }
                    }
                }
            }
        }
    }*/

    /*pub fn clear_cols(&mut self) {
        for combo in self.combos.iter() {
            combo.remove_all();
        }
    }*/

    pub fn get_parent(&self) -> Box {
        self.mapping_box.clone()
    }

    pub fn get_mapping_name(&self) -> String {
        self.mapping_name.clone()
    }

    /// The "design_menu" is the group of widgets that compose a mapping menu
    /// excluding the column combo boxes. The dispatching logic that instantiate
    /// the widgets specific to each mapping is implemented here.
    pub fn build_mapping_design_widgets(
        &mut self,
        builder : &Builder,
        view : Rc<RefCell<PlotView>>
    ) {
        let color_id = self.mapping_type.clone() + "_color_btn";
        let color_btn : ColorButton =
            builder.get_object(&color_id[..]).unwrap();
        connect_update_color_property(
            &color_btn,
            view.clone(),
            self.mapping_name.clone(),
            "color".into(),
            "mapping"
        );
        self.design_widgets.insert(color_id, color_btn.upcast());
        match &self.mapping_type[..] {
            "line" => {
                let width_scale : Scale =
                    builder.get_object("line_width_scale").unwrap();
                connect_update_scale_property(
                    &width_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "width".into(),
                    "mapping"
                );
                self.design_widgets.insert("line_width_scale".into(), width_scale.upcast());
                let dash_scale : Scale =
                    builder.get_object("line_dash_scale").unwrap();
                connect_update_scale_property(
                    &dash_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "dash".into(),
                    "mapping"
                );
                self.design_widgets.insert("line_dash_scale".into(), dash_scale.upcast());
            },
            "scatter" => {
                let radius_scale : Scale =
                    builder.get_object("scatter_radius_scale").unwrap();
                connect_update_scale_property(
                    &radius_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "radius".into(),
                    "mapping"
                );
                self.design_widgets.insert("scatter_radius_scale".into(), radius_scale.upcast());
            },
            "bar" => {
                let anchor_switch : Switch
                    = builder.get_object("bar_anchor_switch").unwrap();
                connect_update_switch_property(
                    &anchor_switch,
                    view.clone(),
                    self.mapping_name.clone(),
                    "center_anchor".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_anchor_switch".into(), anchor_switch.upcast());
            },
            "text" => {

            },
            "area" => {
                let opacity_scale : Scale =
                    builder.get_object("area_opacity_scale").unwrap();
                connect_update_scale_property(
                    &opacity_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "opacity".into(),
                    "mapping"
                );
            },
            "surface" => {
                let opacity_scale : Scale =
                    builder.get_object("surface_opacity_scale").unwrap();
                connect_update_scale_property(
                    &opacity_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "opacity".into(),
                    "mapping"
                );
            },
            _ => { }
        }
    }

    // This, together with build_mapping_layout_menu(), completes the logic
    // of instantiating a mapping menu. This method deals with the mapping-specific
    // combo box text instantiation logic, following the convention of naming
    // all columns by starting with the prefix with the mapping name.
    /*pub fn build_combo_columns_menu(
        builder : &Builder,
        prefix : String
    ) -> Vec<ComboBoxText> {
        let mut combo_ids = Vec::new();
        combo_ids.push(prefix.clone()+"_column_x_combo");
        combo_ids.push(prefix.clone()+"_column_y_combo");
        match &prefix[..] {
            "bar" => {
                combo_ids.push(prefix.clone() + "_column_width_combo");
                combo_ids.push(prefix.clone() + "_column_height_combo");
            },
            "area" => {
                combo_ids.push(prefix.clone() + "_column_ymax_combo");
            },
            "text" => {
                combo_ids.push(prefix.clone() + "_column_text_combo");
            },
            "surface" => {
                combo_ids.push(prefix.clone() + "_column_z_combo");
            },
            _ => { }
        };
        let mut combos : Vec<ComboBoxText> = Vec::new();
        for c in combo_ids.iter() {
            let combo : ComboBoxText = builder.get_object(&c[..]).unwrap();
            combos.push(combo);
        }
        combos
    }*/

    pub fn set_color_property(wid : &Widget, value : &str) {
        let c : ColorButton = wid.clone().downcast()
            .expect("Could not downcast to ColorButton");
        let color : RGBA = value.parse()
            .expect("Could not parse value as RGBA");
        c.set_rgba(&color);
    }

    pub fn set_scale_property(wid : &Widget, value : &str) {
        let s : Scale = wid.clone().downcast()
            .expect("Could not downcast to scale");
        s.get_adjustment().set_value(value.parse().unwrap());
    }

    pub fn set_entry_property(wid : &Widget, value : &str) {
        let e : Entry = wid.clone().downcast()
            .expect("Could not downcast to entry");
        e.set_text(value);
    }

    pub fn update_widget_values(
        &self,
        properties : HashMap<String, String>
    ) -> Result<(), &'static str> {
        let no_wid = "Widget not found";
        let no_val = "Property value not found";
        let wid_col = self.design_widgets.get(&(self.mapping_type.clone() + "_color_btn"))
            .ok_or(no_wid)?;
        Self::set_color_property(wid_col, properties.get("color")
            .ok_or(no_val)?);
        match &(self.mapping_type)[..] {
            "line" => {
                let wid_width = self.design_widgets.get("line_width_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(wid_width, properties.get("width")
                    .ok_or(no_val)?);
                let wid_dash = self.design_widgets.get("line_dash_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(wid_dash, properties.get("dash")
                    .ok_or(no_val)?);
            },
            "scatter" => {
                let wid_radius = self.design_widgets.get("scatter_radius_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(wid_radius, properties.get("radius")
                    .ok_or(no_val)?);
            },
            "text" => {

            },
            "bar" => {

            },
            "area" => {
                let opacity_wid = self.design_widgets.get("area_opacity_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(opacity_wid, properties.get("opacity")
                    .ok_or(no_val)?);
            },
            "surface" => {
                let opacity_wid = self.design_widgets.get("area_opacity_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(opacity_wid, properties.get("opacity")
                    .ok_or(no_val)?);
                let baseline_wid = self.design_widgets.get("surface_baseline_entry")
                    .ok_or(no_wid)?;
                Self::set_entry_property(baseline_wid, properties.get("z_min")
                    .ok_or(no_val)?);
                let maximum_wid = self.design_widgets.get("surface_maximum_entry")
                    .ok_or(no_wid)?;
                Self::set_entry_property(maximum_wid, properties.get("z_max")
                    .ok_or(no_val)?);
                let wid_col_max = self.design_widgets.get(&(self.mapping_type.clone() + "_color_final_btn"))
                    .ok_or(no_wid)?;
                Self::set_color_property(wid_col_max, properties.get("final_color")
                .ok_or(no_val)?);
            },
            _ => {
                return Err("Invalid mapping type");
            }
        }
        Ok(())
    }

    pub fn update_data(
        &self,
        cols : Columns,
        pl_view : &mut PlotView
    ) -> Result<(), &'static str> {
        let (pos0, pos1) = match (cols.try_numeric(0), cols.try_numeric(1)) {
            (Some(c0), Some(c1)) => {
                (c0, c1)
            },
            _ => {
                return Err("Error retrieving two first columns to position");
            }
        };
        let vec_pos = vec![pos0, pos1];
        match &self.mapping_type[..] {
            "text" => {
                if let Some(c) = cols.try_access::<String>(2) {
                    let vec_txt = Vec::from(c);
                    pl_view.update(&mut UpdateContent::TextData(
                        self.mapping_name.clone(),
                        vec_pos,
                        vec_txt
                    ));
                } else {
                    return Err("Error setting third column to text");
                }
            },
            "line" | "scatter" => {
                pl_view.update(&mut UpdateContent::Data(
                    self.mapping_name.clone(),
                    vec_pos
                ));
            },
            mapping => {
                println!("Informed mapping: {}", mapping);
                return Err("Invalid mapping type");
            }
        }
        Ok(())
    }

}

/// Updates the data underlying a single mapping
/// from the plot and queues a redraw. Assumes
/// both the plot and source were unwrapped from
/// Rc<RefCell<.>>
pub fn update_mapping_data(
    source : &TableEnvironment,
    mapping_name : String,
    mapping_type : String,
    cols : Vec<String>,
    plot : &mut PlotView,
    tbl_nb : TableNotebook
) -> Result<(), &'static str> {

    println!("{:?}", tbl_nb.full_selected_cols());
    match &mapping_type[..] {
        "text" => {
            // TODO recover data here
            /*let pos_cols = source.get_subset_cols(vec![cols[0].clone(), cols[1].clone()]);
            let txt_cols = source.subset_cols_as_txt(vec![cols[2].clone()]);
            match (pos_cols, txt_cols) {
                (Ok(pcs), Ok(tcs)) => {
                    if let Some(tc) = tcs.get(0) {
                        plot.update(&mut UpdateContent::TextData(mapping_name, pcs, tc.clone()));
                        Ok(())
                    } else {
                        Err("No text column available")
                    }
                },
                _ => Err("Invalid column selection")
            }*/
        },
        _ => {
            /*let matched_cols = source.get_subset_cols(cols);
            match matched_cols {
                Ok(cols) => {
                    plot.update(&mut UpdateContent::Data(mapping_name, cols));
                    Ok(())
                },
                _ => Err("Not possible to fetch columns")
            }*/
        }
    }
    Ok(())
}

// fn build_query_item(label : &str) {
// let bx = Box::new(Orientation::Horizontal, 0);
// let check = CheckButton::new_with_label(label);
// }
// Use new_line/new_scatter etc to hook up layout-dependent signals.
// actual struct is just used to update the data.





