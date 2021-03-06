use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::environment::TableEnvironment;
use crate::plots::plotview::GroupSplit;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
use std::fs::File;
use std::io::Read;
use super::design_menu::*;
use super::scale_menu::*;
use super::layout_toolbar::*;
use super::mapping_menu::DataSource;
// use super::plot_popover::*;
use std::collections::HashMap;
use crate::utils;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
use super::layout_window::{LayoutWindow, MappingTree};

/// PlotWorkspace encapsulates all plotting-related widgets.
#[derive(Clone)]
pub struct PlotWorkspace {
    pub design_menu : DesignMenu, 
    // pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    pub sidebar_stack : Stack,
    pub pl_view : Rc<RefCell<PlotView>>,
    // pub plot_popover : PlotPopover,
    pub layout_toolbar : LayoutToolbar,
    new_layout_btn : Button,
    glade_def : Rc<HashMap<String, String>>,
    pub layout_window : LayoutWindow,
    pub layout_path : Rc<RefCell<Option<String>>>,
    pub sources : Rc<RefCell<Vec<DataSource>>>
}

impl PlotWorkspace {

    pub fn set_active(&self, state : bool) {
        if state == false {
            self.layout_window.xml_load_dialog.unselect_all();
        }
        if let Some(true) = self.sidebar_stack.get_visible_child_name()
            .map(|n| n.as_str() == "layout" ) {
            self.sidebar_stack.set_visible_child_name("empty");
        } else {
            self.sidebar_stack.set_visible_child_name("database");
        }
    }

    pub fn layout_loaded(&self) -> bool {
        if self.layout_path.borrow().is_some() {
            true
        } else {
            false
        }
    }

    fn build_layout_new_btn(
        builder : Builder,
        sidebar_stack : Stack,
        status_stack : StatusStack,
        clear_layout_btn :ToolButton,
        plot_toggle : ToggleButton,
        layout_path : Rc<RefCell<Option<String>>>
    ) -> Button {
        let new_layout_btn : Button = builder.get_object("layout_new_btn").unwrap();
        let sidebar_stack = sidebar_stack.clone();
        {
            let sidebar_stack = sidebar_stack.clone();
            let status_stack = status_stack.clone();
            let clear_layout_btn = clear_layout_btn.clone();
            new_layout_btn.connect_clicked(move |_btn| {
                sidebar_stack.set_visible_child_name("layout");
                *(layout_path.borrow_mut()) = Some(String::from(
                    "assets/plot_layout/layout-unique.xml"
                ));
                status_stack.try_show_alt();
                clear_layout_btn.set_sensitive(true);
                plot_toggle.set_active(true);
            });
        }
        new_layout_btn
    }

    pub fn build_glade_def() -> Rc<HashMap<String, String>> {
        let mut def_content = HashMap::new();
        for m in ["line", "bar", "area", "scatter", "surface", "text"].iter() {
            let fpath = utils::glade_path(&format!("{}-box.glade", m)[..]).unwrap();
            if let Ok(mut f) = File::open(fpath) {
                let mut glade_str = String::new();
                if let Err(e) = f.read_to_string(&mut glade_str) {
                    panic!("{}", e);
                }
                def_content.insert(format!("{}", m), glade_str);
            } else {
                panic!("Error opening glade definition");
            }
        }
        Rc::new(def_content)
    }

    /// Get the central point of each subplot and the size of the active area,
    /// for showing the plot popover.
    pub fn get_active_coords(pl_view : &PlotView) -> (i32, i32, i32, i32) {

        let rect = pl_view.parent.get_allocation();
        let (horiz_ratio, vert_ratio) = pl_view.aspect_ratio();
        let (horiz_ratio_compl, vert_ratio_compl) = (1. - horiz_ratio, 1. - vert_ratio);
        let (w, h) = (rect.width, rect.height);
        let half_full_horiz = (rect.width as f64 * 0.5) as i32;
        let half_full_vert = (rect.height as f64 * 0.5) as i32;

        let horiz = (w as f64 * horiz_ratio) as i32;
        let horiz_compl = (w as f64 * horiz_ratio_compl) as i32;
        let half_horiz = (0.5*horiz as f64) as i32;
        let half_horiz_compl = horiz+(0.5*horiz_compl as f64) as i32;

        let vert = (h as f64 * vert_ratio) as i32;
        let vert_compl = (h as f64 * vert_ratio_compl) as i32;
        let half_vert = (0.5*vert as f64) as i32;
        let half_vert_compl = vert+(0.5*vert_compl as f64) as i32;

        println!("Allocation: {:?}", (w, h));
        let active_area = pl_view.get_active_area();
        let group_split = pl_view.plot_group.group_split();
        let (x, y) = match pl_view.plot_group.size() {
            1 => (half_full_horiz, half_full_vert),
            2 => match group_split {
                GroupSplit::Horizontal => match active_area {
                    0 => (half_horiz, half_full_vert),
                    1 => (half_horiz_compl, half_full_vert),
                    _ => panic!("Invalid area"),
                },
                GroupSplit::Vertical => match active_area {
                    0 => (half_full_horiz, half_vert),
                    1 => (half_full_horiz, half_vert_compl),
                    _ => panic!("Invalid area"),
                },
                _ => panic!("Invalid split pattern")
            },
            3 => match group_split {
                GroupSplit::ThreeLeft => match active_area {
                    0 => (half_horiz, half_full_vert),
                    1 => (half_horiz_compl, half_vert),
                    2 => (half_horiz_compl, half_vert_compl),
                    _ => panic!("Invalid area"),
                },
                GroupSplit::ThreeTop => match active_area {
                    0 => (half_full_horiz, half_vert),
                    1 => (half_horiz, half_vert_compl),
                    2 => (half_horiz_compl, half_vert_compl),
                    _ => panic!("Invalid area"),
                },
                GroupSplit::ThreeRight => match active_area {
                    0 => (half_horiz, half_vert),
                    1 => (half_horiz_compl, half_full_vert),
                    2 => (half_horiz, half_vert_compl),
                    _ => panic!("Invalid area"),
                },
                GroupSplit::ThreeBottom => match active_area {
                    0 => (half_horiz, half_vert),
                    1 => (half_horiz_compl, half_vert),
                    2 => (half_full_horiz, half_vert_compl),
                    _ => panic!("Invalid area"),
                },
                _ => panic!("Invalid split pattern")
            },
            4 => match active_area {
                0 => (half_horiz, half_vert),
                1 => (half_horiz_compl, half_vert),
                2 => (half_horiz, half_vert_compl),
                3 => (half_horiz_compl, half_vert_compl),
                _ => panic!("Invalid area"),
            },
            _ => panic!("Invalid plot index")
        };
        println!("Active area: {:?}", (x, y, w, h));
        (x, y, w, h)
    }

    fn updated_active_area(pl_view : &PlotView, x : i32, y : i32, w : i32, h : i32) -> usize {
        let (horiz_ratio, vert_ratio) = pl_view.aspect_ratio();
        let x_left = x < (w as f64*horiz_ratio) as i32;
        let y_top = y < (h as f64*vert_ratio) as i32;
        match (pl_view.plot_group.size(), pl_view.plot_group.group_split()) {
            (1, _) => 0,
            (2, GroupSplit::Horizontal) => if x_left { 0 } else { 1 },
            (2, GroupSplit::Vertical) => if y_top { 0 } else { 1 },
            (3, GroupSplit::ThreeLeft) => match (x_left, y_top) {
                (true, _) => 0,
                (false, true) => 1,
                (false, false) => 2,
            },
            (3, GroupSplit::ThreeTop) => match (x_left, y_top) {
                (_, true) => 0,
                (true, false) => 1,
                (false, false) => 2,
            },
            (3, GroupSplit::ThreeRight) => match (x_left, y_top) {
                (true, true) => 0,
                (false, _) => 1,
                (true, false) => 2,
            },
            (3, GroupSplit::ThreeBottom) => match (x_left, y_top) {
                (true, true) => 0,
                (false, true) => 1,
                (_, false) => 2,
            },
            (4, _) => match (x_left, y_top) {
                    (true, true) => 0,
                    (false, true) => 1,
                    (true, false) => 2,
                    (false, false) => 3,
            },
            _ => panic!("Undefined plot size")
        }
    }

    pub fn new(
        builder : Builder,
        pl_view : Rc<RefCell<PlotView>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        plot_toggle : ToggleButton,
        table_toggle : ToggleButton,
        sidebar_stack : Stack,
    ) -> Self {
        // let mapping_menus = Rc::new(RefCell::new(Vec::new()));
        let design_menu = build_design_menu(&builder, pl_view.clone());
        let glade_def = Self::build_glade_def();
        let layout_path = Rc::new(RefCell::new(None));
        let plot_ev : EventBox = builder.get_object("plot_ev").unwrap();
        let sel_mapping = Rc::new(RefCell::new(String::new()));
        // let plot_popover = PlotPopover::new(&builder);

        /*{
            let pl_view = pl_view.clone();
            let plot_popover = plot_popover.clone();
            let scale_menus = scale_menus.clone();
            plot_ev.connect_button_press_event(move |wid, ev| {
                let (x, y) = ev.get_position();
                let w = wid.get_allocation().width;
                let h = wid.get_allocation().height;
                let (ar, info_x, info_y) = if let Ok(mut pl) = pl_view.try_borrow_mut() {
                    let new_ix = Self::updated_active_area(&*pl, x as i32, y as i32, w, h);
                    println!("New Active area: {}", new_ix);
                    pl.change_active_area(new_ix);
                    plot_popover.set_active_first_mapping(new_ix);
                    println!("Before update: {:?}", pl.current_scale_info("x"));
                    (pl.aspect_ratio(), pl.current_scale_info("x"), pl.current_scale_info("y"))
                } else {
                    println!("Failed acquiring mutable reference to plot view/selected mapping");
                    return glib::signal::Inhibit(true);
                };
                scale_menus.0.update(info_x.clone());
                scale_menus.1.update(info_y);
                println!("After update: {:?}", info_x);
                println!("Draw area touched at {:?}", (x, y));
                plot_popover.show_from_click(
                    &ev,
                    w,
                    h,
                    pl_view.borrow().group_split(),
                    pl_view.borrow().get_active_area(),
                    ar
                );
                println!("After click: {:?}", info_x);
                glib::signal::Inhibit(true)
            });
        }*/
        
        let sources = Rc::new(RefCell::new(Vec::new()));
        let layout_toolbar = LayoutToolbar::build(
            builder.clone(),
            status_stack.clone(),
            sidebar_stack.clone(),
            pl_view.clone(),
            // mapping_menus.clone(),
            // plot_popover.clone(),
            sources.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            layout_path.clone(),
            plot_toggle.clone(),
            glade_def.clone(),
            sel_mapping.clone()
        );
        let layout_window = LayoutWindow::new(
            builder.clone(),
            pl_view.clone(),
            // mapping_stack.clone(),
            // mapping_menus.clone(),
            // plot_popover.mapping_stack.clone(),
            layout_path.clone(),
            // design_menu.clone(),
            // scale_menus.clone(),
            layout_toolbar.group_toolbar.clone()
        );
        layout_toolbar.connect_add_mapping_clicked(
            // plot_popover.clone(),
            glade_def.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            pl_view.clone(),
            // mapping_menus.clone(),
            status_stack.clone(),
            sel_mapping.clone(),
            layout_window.mapping_tree.clone(),
            sources.clone()
        );
        layout_toolbar.connect_edit_mapping_clicked(
            plot_toggle.clone(),
            // plot_popover.clone(),
            layout_window.clone(),
            pl_view.clone(),
            tbl_nb.clone()
        );
        layout_toolbar.connect_remove_mapping_clicked(
            // mapping_menus.clone(),
            pl_view.clone(),
            // plot_popover.clone(),
            status_stack.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            layout_toolbar.mapping_popover.clone(),
            layout_window.mapping_tree.clone(),
            sources.clone()
        );
        let new_layout_btn = Self::build_layout_new_btn(
            builder.clone(),
            sidebar_stack.clone(),
            status_stack.clone(),
            layout_toolbar.clear_layout_btn.clone(),
            plot_toggle.clone(),
            layout_path.clone()
        );
        LayoutWindow::connect_layout_load(
            glade_def.clone(),
            builder.clone(),
            pl_view.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            status_stack.clone(),
            // plot_popover.clone(),
            // mapping_menus.clone(),
            sources.clone(),
            design_menu.clone(),
            // (scale_menus.0.clone(), scale_menus.1.clone()),
            plot_toggle,
            layout_window.clone(),
            layout_path.clone(),
            (layout_window.horiz_ar_scale.clone(), layout_window.vert_ar_scale.clone()),
            layout_toolbar.group_toolbar.clone(),
            layout_window.mapping_tree.clone()
        );

        // TODO this logic should be moved to a button at the bottom of all mappings
        // at the new MappingTree structure.
        /*{
            // let mapping_menus = mapping_menus.clone();
            let tbl_nb = tbl_nb.clone();
            let plot_popover = plot_popover.clone();
            let layout_toolbar = layout_toolbar.clone();
            plot_popover.tbl_btn.clone().connect_clicked(move |_btn| {
                if let Some(mapping_ix) = plot_popover.get_selected_mapping() {
                    layout_toolbar.update_selected_mapping(tbl_nb.clone(), /*mapping_menus.clone()*/ mapping_ix);
                    table_toggle.set_active(true);
                } else {
                    println!("No selected mapping");
                }
            });
        }*/
        
        if let Ok(pl_view) = pl_view.try_borrow() {
            if !pl_view.parent.get_realized() {
                // Confirm the parent has been set to visible, or else plot_popover
                // will appear at the wrong position if the plot stack child hasn't been
                // selected before its first appearance.
                pl_view.parent.realize();
            }
        } else {
            println!("Failed acquiring reference to plot view");
        }
        let ws = Self {
            design_menu,
            // scale_menus,
            // mapping_menus,
            sidebar_stack : sidebar_stack.clone(),
            pl_view : pl_view.clone(),
            layout_toolbar,
            glade_def,
            new_layout_btn,
            // load_layout_btn,
            // xml_load_dialog,
            layout_window,
            layout_path,
            // plot_popover,
            sources
        };
        ws.layout_window.connect_clear(&ws);
        ws
    }

    /*/// Add mapping from a type string description, attributing to its
    /// name the number of mappings currently used. Used when the user
    /// already selected some columns and want to create a new mapping.
    pub fn add_mapping_from_type(
        glade_def : Rc<HashMap<String, String>>,
        mapping_type : &str,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        status_stack : StatusStack,
        active_area : usize
    ) {
        println!("Adding mapping of type {} to active area {}", mapping_type, active_area);
        let name = if let Ok(menus) = mapping_menus.try_borrow() {
            let active = plot_view.borrow().get_active_area();
            let mut n : usize = 0;
            for m in menus.iter() {
                if m.plot_ix == active {
                    n += 1;
                }
            }
            format!("{}", n)
        } else {
            println!("Unable to get reference to mapping menus");
            return;
        };
        /*let menu = MappingMenu::create(
            glade_def.clone(),
            Rc::new(RefCell::new(name)),
            mapping_type.to_string(),
            data_source.clone(),
            plot_view.clone(),
            None,
            active_area
        );
        match menu {
            Ok(m) => {
                Self::append_mapping_menu(
                    m,
                    mapping_menus.clone(),
                    plot_popover.clone(),
                    status_stack.clone(),
                    plot_view.clone(),
                    data_source.clone(),
                    tbl_nb.clone(),
                    None,
                    true
                );
            },
            Err(e) => { println!("{}", e); return; }
        }*/
    }*/

    /*pub fn clear_mapping_data(&self) -> Result<(), &'static str> {
        /*match (self.pl_view.try_borrow_mut(), self.mapping_menus.try_borrow()) {
            (Ok(mut pl_view), Ok(mappings)) => {
                for m in mappings.iter() {
                    if let Err(e) = m.clear_data(&mut pl_view) {
                        println!("{}", e);
                    }
                }
                Ok(())
            },
            _ => {
                Err("Unable to retrieve mutable reference to pl view/reference to mappings")
            }
        }*/
    }*/

    /*pub fn update_mapping_widgets(
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        glade_def : Rc<HashMap<String, String>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
    ) {
        Self::clear_mapping_widgets(
            mapping_menus.clone(),
            plot_popover.mapping_stack.clone()
        ).expect("Error clearing mappings");
        let n_plots = plot_view.borrow().n_plots();
        for plot_ix in 0..n_plots {
            let new_info = match plot_view.try_borrow_mut() {
                Ok(mut pl_view) => { pl_view.change_active_area(plot_ix); pl_view.mapping_info() },
                Err(e) => { println!("{}", e); return; }
            };
            for m_info in new_info.iter() {
                /*let menu = MappingMenu::create(
                    glade_def.clone(),
                    Rc::new(RefCell::new(m_info.0.clone())),
                    m_info.1.clone(),
                    data_source.clone(),
                    plot_view.clone(),
                    Some(m_info.2.clone()),
                    plot_ix
                );
                match menu {
                    Ok(m) => {
                        Self::append_mapping_menu(
                            m,
                            mapping_menus.clone(),
                            //plot_notebook.clone(),
                            plot_popover.clone(),
                            status_stack.clone(),
                            plot_view.clone(),
                            data_source.clone(),
                            tbl_nb.clone(),
                            None,
                            false
                        );
                    },
                    Err(e) => { println!("{}", e); return; }
                }*/
            }
        }
        plot_view.borrow_mut().change_active_area(0);
    }*/

    pub fn update_mapping_data(
        &self,
        t_env : &TableEnvironment,
        status_stack : StatusStack
    ) -> Result<(), &'static str> {
        let mut pl = self.pl_view.try_borrow_mut()
            .map_err(|_| "Could not get mutable reference to plot view")?;
        //let menus = self.mapping_menus.try_borrow()
        //    .map_err(|_| "Could not get reference to mapping menus" )?;
        for source in self.sources.borrow().iter() {
            if let Err(e) = Self::update_data(&source, t_env, &mut pl) {
                status_stack.update(Status::SqlErr(format!("{}", e)));
                return Err("Error updating mappings");
            }
        }
        status_stack.update(Status::Ok);
        Ok(())
    }

    /*fn append_mapping_menu(
        m : MappingMenu,
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        status_stack : StatusStack,
        plot_view : Rc<RefCell<PlotView>>,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        pos : Option<usize>,
        with_data : bool
    ) {
        match (plot_view.try_borrow_mut(), tbl_env.try_borrow(), mappings.try_borrow_mut()) {
            (Ok(mut pl), Ok(t_env), Ok(mut mappings)) => {
                match pos {
                    Some(p) => mappings.insert(p, m.clone()),
                    None => mappings.push(m.clone())
                }
                let inserted_pos = pos.unwrap_or(mappings.len() - 1);
                println!("Adding mapping to {} pos of mapping vector (plot {})", inserted_pos, m.plot_ix);
                plot_popover.add_mapping(&m, inserted_pos);
                if let Ok(name) = m.mapping_name.try_borrow() {
                    pl.update(&mut UpdateContent::NewMapping(
                        name.clone(),
                        m.mapping_type.to_string(),
                        m.plot_ix
                    ));
                    println!("Mapping appended: {:?}", m);
                    if with_data {
                        if let Err(e) = m.reassign_data(tbl_nb.full_selected_cols(), &t_env, &mut pl) {
                            status_stack.update(Status::SqlErr(format!("{}", e)));
                            return;
                        }
                    } else {
                        if let Err(e) = m.clear_data(&mut pl) {
                            println!("{}", e);
                        }
                    }
                } else {
                    println!("Unable to retrive reference to mapping name");
                }
            },
            (_,_,Err(e)) => { println!("{}", e); },
            _ => {
                println!("Unable to retrieve mutable reference to plot view|data source");
            }
        }
    }*/

    /// Clear mappings and layout to the first (unique) layout
    pub fn clear(&self) {
        if let Err(e) = self.clear_mappings() {
            println!("{}", e);
        };
        if let Ok(mut pl_view) = self.pl_view.try_borrow_mut() {
            pl_view.change_active_area(0);
            pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-unique.xml")));
        } else {
            println!("Failed to borrow mutable reference to plotview.");
        }
    }

    /// Clear only mappings, preserving the layout. Should be called
    /// at the moment a new layout is loaded (at self.clear) or else
    /// the XML will be in an invalid state. Also called whenever the
    /// query sequence that is being used is changed, or else the user
    /// would end up in with a plot that is not in the table environment.
    pub fn clear_mappings(&self) -> Result<(), &'static str> {
        if let Ok(mut pl_view) = self.pl_view.try_borrow_mut() {
            pl_view.update(&mut UpdateContent::Erase);
        } else {
            return Err("Unable to borrow plotview when clearing mappings");
        }
        /*Self::clear_mapping_widgets(
            self.mapping_menus.clone(),
            self.plot_popover.mapping_stack.clone()
        )?;*/
        // self.plot_popover.clear();
        Ok(())
    }

    /*/// Erases all mapping widgets. Should be called
    /// at the moment a new layout should be loaded (at self.clear) or else
    /// the widgets will be in an invalid state with the XML.
    pub fn clear_mapping_widgets(
        // pl_view : Rc<RefCell<PlotView>>,
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        mapping_stack : Stack,
    ) -> Result<(), &'static str> {
        if let Ok(mut mappings) = mappings.try_borrow_mut() {
            for m in mappings.iter() {
                //plot_notebook.remove(&m.get_parent());
                mapping_stack.remove(&m.get_parent());
            }
            mappings.clear();
            Ok(())
        } else {
            Err("Could not fetch mutable reference to mapping menus before clearing them")
        }
    }*/
    
    pub fn update_source(source : &mut DataSource, new_ixs : Vec<usize>, t_env : &TableEnvironment) -> Result<(), &'static str> {   
        if !t_env.preserved_since(source.hist_ix) {
            source.valid = false;
            println!("History index for current mapping: {}", source.hist_ix);
            return Err("Environment was updated and data is no longer available");
        }
        source.ixs.clear();
        source.ixs.extend(new_ixs.clone());
        let (col_names, tbl_ix, query) = t_env.get_column_names(&new_ixs[..])
            .ok_or("Unable to retrieve table data")?;
        // for (name, lbl) in source.col_names.iter().zip(self.column_labels.iter()) {
        //    lbl.set_text(&name[..]);
        // }
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
    }
    
    /// Updates data from a table enviroment and the saved column indices.
    pub fn update_data(
        source : &DataSource,
        t_env : &TableEnvironment, 
        pl_view : &mut PlotView
    ) -> Result<(), &'static str> {
        let selected = source.ixs.clone();
        if selected.len() == 0 {
            println!("No data for current mapping");
            return Ok(())
        }
        let (cols, _, query) = t_env.get_columns(&selected[..]).unwrap();
        //let name = self.get_mapping_name()
        //    .map(|n| n.clone())
        //    .ok_or("Unable to get mapping name")?;
        let pos0 = cols.try_numeric(0)
            .ok_or("Error mapping column 1 to position")?;
        // let col_names : Vec<_> = cols.names().iter()
        //    .map(|n| n.to_string())
        //    .collect();
        pl_view.update(&mut UpdateContent::ColumnNames(source.name.clone(), source.col_names.clone()));
        pl_view.update(&mut UpdateContent::Source(source.name.clone(), query));
        match &source.ty[..] {
            "text" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to position")?;
                if let Some(c) = cols.try_access::<String>(2) {
                    let vec_txt = Vec::from(c);
                    pl_view.update(&mut UpdateContent::TextData(
                        source.name.clone(),
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
                    source.name.clone(),
                    vec![pos0, pos1]
                ));
            },
            "bar" => {
                pl_view.update(&mut UpdateContent::Data(
                    source.name.clone(),
                    vec![pos0]
                ));
            },
            "area" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to y inferior limit")?;
                let pos2 = cols.try_numeric(2).ok_or("Error mapping column 3 to y superior limit")?;
                pl_view.update(&mut UpdateContent::Data(
                    source.name.clone(),
                    vec![pos0, pos1, pos2]
                ));
            },
            "surface" => {
                let pos1 = cols.try_numeric(1).ok_or("Error mapping column 2 to y inferior limit")?;
                let density = cols.try_numeric(2).ok_or("Error mapping column 3 to density")?;
                pl_view.update(&mut UpdateContent::Data(
                    source.name.clone(),
                    vec![pos0, pos1, density]
                ));
            },
            mapping => {
                println!("Informed mapping: {}", mapping);
                return Err("Invalid mapping type");
            }
        }
        // self.set_sensitive(true);
        Ok(())
    }


}
