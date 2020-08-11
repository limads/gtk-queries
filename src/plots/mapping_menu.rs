use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
use crate::plots::layout_aux::*;
use crate::tables::{ /*source::EnvironmentSource,*/ environment::TableEnvironment};
use std::collections::HashMap;
use gdk::RGBA;
use gtk::prelude::*;
use std::default::Default;

#[derive(Clone, Debug, Default)]
pub struct DataSource {

    /// Table position in the environment.
    pub tbl_pos : Option<usize>,

    /// Linear index, from first column of first table to last column of last table
    pub ixs : Vec<usize>,

    /// Point in the table environment index history at which the data was fetch.
    pub hist_ix : usize,

    /// If environment source was updated and data is no longer present, set this to false.
    pub valid : bool,

    /// Table index, from first column of the current table.
    pub tbl_ixs : Vec<usize>,
    pub col_names : Vec<String>,
    pub query : String
}

/// MappingMenu is the structure common across all menus
/// used to manipulate the mappings directly (line, trace, scatter).
#[derive(Clone, Debug)]
pub struct MappingMenu {
    pub mapping_name : Rc<RefCell<String>>,
    pub mapping_type : String,
    pub mapping_box : Box,
    //pub combos : Vec<ComboBoxText>,
    pub design_widgets : HashMap<String, Widget>,
    pub column_labels : Vec<Label>,
    pub source : Rc<RefCell<DataSource>>,
    pub plot_ix : usize,
    pub tab_img : Image
}

// If all combo boxes have valid textual entries, return Some<Vec>.
// Return None otherwise.
impl MappingMenu {

    pub fn create_tab_image(m_type : String) -> Image {
        let tab_img_path = String::from("assets/icons/") + &m_type + ".svg";
        Image::new_from_file(&tab_img_path[..])
    }

    /// The creation of a mapping menu is based on an id naming convention
    /// of passing a prefix identifying the mappping (line, scatter, box, etc)
    /// followed by an element identifier. This convention applies to the enclosing box
    /// (line_box, scatter_box ...) and its constituint widgets (scatter_color_button,
    /// line_color_button) and so on. The builder for each mapping menu must be unique
    /// to avoid aliasing.
    /// Make this mapping_menu::create(.)
    pub fn create(
        glade_def : Rc<HashMap<String, String>>,
        mapping_name : Rc<RefCell<String>>,
        mapping_type : String,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        pl_view : Rc<RefCell<PlotView>>,
        properties : Option<HashMap<String, String>>
    ) -> Result<MappingMenu, &'static str> {
        let builder = Builder::new_from_string(&glade_def[&mapping_type][..]);
        let valid_mappings = ["line", "scatter", "bar", "area", "text", "surface"];
        if !valid_mappings.iter().any(|s| &mapping_type[..] == *s) {
            return Err("Invalid mapping type. Must be line|scatter|bar|area|text|surface");
        }
        let box_name = mapping_type.clone() + "_box";
        let mapping_box : Box = builder.get_object(&box_name).unwrap();
        let design_widgets = HashMap::new();
        let mut source_content : DataSource = Default::default();
        source_content.hist_ix = tbl_env.borrow().current_hist_index();
        let source = Rc::new(RefCell::new(source_content));
        let plot_ix = pl_view.borrow().get_active_area();
        let tab_img = MappingMenu::create_tab_image(mapping_type.clone());
        let mut m = MappingMenu {
            mapping_name,
            mapping_type,
            mapping_box,
            design_widgets,
            source,
            plot_ix,
            tab_img,
            column_labels : Vec::new()
        };
        m.build_mapping_design_widgets(
            &builder,
            pl_view.clone()
        );
        m.build_column_labels(
            &builder
        );
        if let Some(prop) = properties {
            if let Err(e) = m.update_widget_values(prop) {
                println!("{}", e);
            }
        }
        Ok(m)
    }

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

    pub fn get_mapping_name(&self) -> Option<String> {
        match self.mapping_name.try_borrow() {
            Ok(n) => Some(n.clone()),
            Err(_) => { println!("Failed to retrieve mutable reference to mapping index"); None }
        }
    }

    pub fn set_mapping_name(&self, new_name : String) {
        match self.mapping_name.try_borrow_mut() {
            Ok(mut name) => *name = new_name,
            Err(_) => println!("Unable to retrieve mutable reference to mapping name")
        }
    }

    pub fn build_column_labels(&mut self, builder : &Builder) {
        match &self.mapping_type[..] {
            "line" => {
                self.column_labels.push(builder.get_object::<Label>("column_x_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_y_label").unwrap());
            },
            "scatter" => {
                self.column_labels.push(builder.get_object::<Label>("column_x_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_y_label").unwrap());
            },
            "bar" => {
                self.column_labels.push(builder.get_object::<Label>("column_height_label").unwrap());
            },
            "text" => {
                self.column_labels.push(builder.get_object::<Label>("column_x_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_y_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_text_label").unwrap());
            },
            "area" => {
                self.column_labels.push(builder.get_object::<Label>("column_x_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_ymin_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_ymax_label").unwrap());
            },
            "surface" => {
                self.column_labels.push(builder.get_object::<Label>("column_x_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_y_label").unwrap());
                self.column_labels.push(builder.get_object::<Label>("column_z_label").unwrap());
            },
            _ => {
                panic!("Unknown surface: {}", self.mapping_type);
            }
        }
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
                let horizontal_switch : Switch
                    = builder.get_object("bar_horizontal_switch").unwrap();
                connect_update_switch_property(
                    &horizontal_switch,
                    view.clone(),
                    self.mapping_name.clone(),
                    "horizontal".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_horizontal_switch".into(), horizontal_switch.upcast());
                let width_scale : Scale =
                    builder.get_object("bar_width_scale").unwrap();
                connect_update_scale_property(
                    &width_scale,
                    view.clone(),
                    self.mapping_name.clone(),
                    "bar_width".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_width_scale".into(), width_scale.upcast());

                // TODO panicking here (bar_origin_x_entry not fond).
                let origin_x_entry : Entry =
                    builder.get_object("bar_origin_x_entry").unwrap();
                connect_update_entry_property(
                    &origin_x_entry,
                    view.clone(),
                    self.mapping_name.clone(),
                    "origin_x".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_origin_x_entry".into(), origin_x_entry.upcast());
                let origin_y_entry : Entry =
                    builder.get_object("bar_origin_y_entry").unwrap();
                connect_update_entry_property(
                    &origin_y_entry,
                    view.clone(),
                    self.mapping_name.clone(),
                    "origin_y".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_origin_y_entry".into(), origin_y_entry.upcast());
                let spacing_entry : Entry =
                    builder.get_object("bar_spacing_entry").unwrap();
                connect_update_entry_property(
                    &spacing_entry,
                    view.clone(),
                    self.mapping_name.clone(),
                    "bar_spacing".into(),
                    "mapping"
                );
                self.design_widgets.insert("bar_spacing_entry".into(), spacing_entry.upcast());
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
                self.design_widgets.insert("area_opacity_scale".into(), opacity_scale.upcast());
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
                self.design_widgets.insert("surface_opacity_scale".into(), opacity_scale.upcast());
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

    pub fn set_switch_property(wid : &Widget, value : &str) {
        let s : Switch = wid.clone().downcast()
            .expect("Could not downcast to entry");
        s.set_active(value.parse().unwrap());
    }

    pub fn set_font_property(wid : &Widget, value : &str) {
        let btn : FontButton = wid.clone().downcast()
            .expect("Could not downcast to entry");
        btn.set_font_name(&value);
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
                let wid_font = self.design_widgets.get("text_mapping_font_btn")
                    .ok_or(no_wid)?;
                Self::set_font_property(wid_font, properties.get("font")
                    .ok_or(no_val)?);
            },
            "bar" => {
                let wid_center = self.design_widgets.get("bar_anchor_switch")
                    .ok_or(no_wid)?;
                Self::set_switch_property(wid_center, properties.get("center_anchor")
                    .ok_or(no_val)?);
                let wid_width = self.design_widgets.get("bar_width_scale")
                    .ok_or(no_wid)?;
                Self::set_scale_property(wid_width, properties.get("bar_width")
                    .ok_or(no_val)?);
                let wid_orig_x = self.design_widgets.get("bar_origin_x_entry")
                    .ok_or(no_wid)?;
                Self::set_entry_property(wid_orig_x, properties.get("origin_x")
                    .ok_or(no_val)?);
                let wid_orig_y = self.design_widgets.get("bar_origin_y_entry")
                    .ok_or(no_wid)?;
                Self::set_entry_property(wid_orig_y, properties.get("origin_y")
                    .ok_or(no_val)?);
                let wid_spacing = self.design_widgets.get("bar_spacing_entry")
                    .ok_or(no_wid)?;
                Self::set_entry_property(wid_spacing, properties.get("bar_spacing")
                    .ok_or(no_val)?);
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

    /// Clear saved column indices and full data.
    pub fn clear_data(&self, pl_view : &mut PlotView) -> Result<(), &'static str> {
        let name = self.get_mapping_name().map(|n| n.clone())
            .ok_or("Unable to get mapping name")?;
        match &self.mapping_type[..] {
            "text" => {
                pl_view.update(&mut UpdateContent::TextData(
                    name.clone(),
                    vec![Vec::new(), Vec::new()],
                    Vec::new()
                ));
            },
            "line" | "scatter" => {
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![Vec::new(), Vec::new()]
                ));
            },
            "bar" => {
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![Vec::new()]
                ));
            },
            "area" | "surface" => {
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![Vec::new(), Vec::new(), Vec::new()]
                ));
            },
            mapping => {
                println!("Informed mapping: {}", mapping);
                return Err("Invalid mapping type");
            }
        }
        if let Ok(mut source) = self.source.try_borrow_mut() {
            source.ixs.clear();
            source.tbl_pos = None;
            source.tbl_ixs.clear();
            source.query.clear();
            source.col_names.clone();
        } else {
            println!("Could not get mutable reference to table source");
        }
        self.set_sensitive(false);
        Ok(())
    }

    /// Updates the source columns then updates the data from the table environment.
    pub fn reassign_data(
        &self,
        cols : Vec<usize>,
        t_env : &TableEnvironment,
        pl_view : &mut PlotView
    ) -> Result<(), &'static str> {
        self.update_source(cols, &t_env)?;
        self.update_data(&t_env, pl_view)
    }

    pub fn update_source(&self, new_ixs : Vec<usize>, t_env : &TableEnvironment) -> Result<(), &'static str> {
        if let Ok(mut source) = self.source.try_borrow_mut() {
            if !t_env.preserved_since(source.hist_ix) {
                source.valid = false;
                println!("History index for current mapping: {}", source.hist_ix);
                return Err("Environment was updated and data is no longer available");
            }
            source.ixs.clear();
            source.ixs.extend(new_ixs.clone());
            let (col_names, tbl_ix, query) = t_env.get_column_names(&new_ixs[..])
                .ok_or("Unable to retrieve table data")?;
            for (name, lbl) in col_names.iter().zip(self.column_labels.iter()) {
                lbl.set_text(&name[..]);
            }
            source.col_names = col_names;
            source.query = query;
            source.tbl_pos = Some(tbl_ix);
            source.hist_ix = t_env.current_hist_index();
            source.valid = true;
            if let Some((_, new_tbl_ixs)) = t_env.global_to_tbl_ix(&new_ixs[..]) {
                source.tbl_ixs.clear();
                source.tbl_ixs.extend(new_tbl_ixs);
            } else {
                return Err("Failed to convert global to local indices");
            }
            println!("Column names : {:?}", source.col_names);
            println!("Linear indices : {:?}", source.ixs);
            println!("Table indices : {:?}", source.tbl_ixs);
            println!("History index : {:?}", source.hist_ix);
            Ok(())
        } else {
            Err("Failed to get mutable reference to table source")
        }
    }

    pub fn set_sensitive(&self, sensitive : bool) {
        for (_, w) in self.design_widgets.iter() {
            if sensitive && !w.is_sensitive() {
                w.set_sensitive(true);
            }
            if !sensitive && w.is_sensitive() {
                w.set_sensitive(false);
            }
        }
    }

    /// Updates data from a table enviroment and the saved column indices.
    pub fn update_data(&self, t_env : &TableEnvironment, pl_view : &mut PlotView) -> Result<(), &'static str> {
        let selected = self.source.try_borrow()
            .map(|source| source.ixs.clone() )
            .map_err(|_| "Unable to retrieve reference to used indices")?;
        if selected.len() == 0 {
            println!("No data for current mapping");
            return Ok(())
        }
        let (cols, _, query) = t_env.get_columns(&selected[..]).unwrap();
        let name = self.get_mapping_name()
            .map(|n| n.clone())
            .ok_or("Unable to get mapping name")?;
        let pos0 = cols.try_numeric(0)
            .ok_or("Error mapping column 1 to position")?;
        let col_names : Vec<_> = cols.names().iter()
            .map(|n| n.to_string())
            .collect();
        pl_view.update(&mut UpdateContent::ColumnNames(name.clone(), col_names));
        pl_view.update(&mut UpdateContent::Source(name.clone(), query));
        match &self.mapping_type[..] {
            "text" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to position")?;
                if let Some(c) = cols.try_access::<String>(2) {
                    let vec_txt = Vec::from(c);
                    pl_view.update(&mut UpdateContent::TextData(
                        name.clone(),
                        vec![pos0, pos1],
                        vec_txt
                    ));
                } else {
                    return Err("Error setting third column to text");
                }
            },
            "line" | "scatter" => {
                let pos1 = cols.try_numeric(1).ok_or("Error retrieving second column to position")?;
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![pos0, pos1]
                ));
            },
            "bar" => {
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![pos0]
                ));
            },
            "area" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to y inferior limit")?;
                let pos2 = cols.try_numeric(2).ok_or("Error mapping column 3 to y superior limit")?;
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![pos0, pos1, pos2]
                ));
            },
            "surface" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to y inferior limit")?;
                let density = cols.try_numeric(2).ok_or("Error mapping column 3 to density")?;
                pl_view.update(&mut UpdateContent::Data(
                    name.clone(),
                    vec![pos0, pos1, density]
                ));
            },
            mapping => {
                println!("Informed mapping: {}", mapping);
                return Err("Invalid mapping type");
            }
        }
        self.set_sensitive(true);
        Ok(())
    }

}




