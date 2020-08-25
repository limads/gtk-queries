use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::{ environment::TableEnvironment };
use crate::plots::plotview::GroupSplit;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
use std::fs::File;
use std::io::Read;
use super::design_menu::*;
use super::scale_menu::*;
use super::layout_toolbar::*;
use super::mapping_menu::{*, MappingMenu};
use super::plot_popover::*;
use std::collections::HashMap;
use crate::utils;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
use std::default::Default;
use super::layout_window::LayoutWindow;

/// PlotWorkspace encapsulates all plotting-related widgets.
#[derive(Clone)]
pub struct PlotWorkspace {
    pub design_menu : DesignMenu,
    pub scale_menus : (ScaleMenu, ScaleMenu),
    pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    pub sidebar_stack : Stack,
    pub pl_view : Rc<RefCell<PlotView>>,
    pub plot_popover : PlotPopover,
    pub layout_toolbar : LayoutToolbar,
    new_layout_btn : Button,
    glade_def : Rc<HashMap<String, String>>,
    pub layout_window : LayoutWindow,
    pub layout_path : Rc<RefCell<Option<String>>>
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

    pub fn get_active_coords(pl_view : &PlotView) -> (i32, i32, i32, i32) {
        let rect = pl_view.parent.get_allocation();
        let (w, h) = (rect.width, rect.height);
        println!("Allocation: {:?}", (w, h));
        let (x, y) = match pl_view.plot_group.size() {
            1 => ((w as f64 * 0.5) as i32, (h as f64 * 0.5) as i32),
            2 => match pl_view.plot_group.group_split() {
                GroupSplit::Horizontal => match pl_view.get_active_area() {
                    0 => ((w as f64 * 0.5) as i32, (h as f64 * 0.33) as i32),
                    1 => ((w as f64 * 0.5) as i32, (h as f64 * 0.66) as i32),
                    _ => panic!("Invalid area"),
                },
                GroupSplit::Vertical => match pl_view.get_active_area() {
                    0 => ((w as f64 * 0.33) as i32, (h as f64 * 0.5) as i32),
                    1 => ((w as f64 * 0.66) as i32, (h as f64 * 0.5) as i32),
                    _ => panic!("Invalid area"),
                },
                _ => panic!("Invalid split pattern")
            },
            4 => match pl_view.get_active_area() {
                0 => ((w as f64 * 0.25) as i32, (h as f64 * 0.25) as i32),
                1 => ((w as f64 * 0.75) as i32, (h as f64 * 0.25) as i32),
                2 => ((w as f64 * 0.25) as i32, (h as f64 * 0.75) as i32),
                3 => ((w as f64 * 0.75) as i32, (h as f64 * 0.75) as i32),
                _ => panic!("Invalid area"),
            },
            _ => panic!("Invalid plot index")
        };
        println!("Active area: {:?}", (x, y, w, h));
        (x, y, w, h)
    }

    // Substitute /2 by *ratio where ratio is the 0-1 aspect ratio set at the layout options
    fn updated_active_area(pl_view : &PlotView, x : i32, y : i32, w : i32, h : i32) -> usize {
        match (pl_view.plot_group.size(), pl_view.plot_group.group_split()) {
            (1, _) => 0,
            (2, GroupSplit::Horizontal) => if y < h / 2 { 0 } else { 1 },
            (2, GroupSplit::Vertical) => if x < w / 2 { 0 } else { 1 },
            (3, GroupSplit::ThreeLeft) => match (x < w / 2, y < h / 2) {
                (true, _) => 0,
                (false, true) => 1,
                (false, false) => 2,
            },
            (3, GroupSplit::ThreeTop) => match (x < w / 2, y < h / 2) {
                (_, true) => 0,
                (true, false) => 1,
                (false, false) => 2,
            },
            (3, GroupSplit::ThreeRight) => match (x < w / 2, y < h / 2) {
                (true, false) => 0,
                (false, _) => 1,
                (true, true) => 2,
            },
            (3, GroupSplit::ThreeBottom) => match (x < w / 2, y < h / 2) {
                (true, false) => 0,
                (false, false) => 1,
                (_, true) => 2,
            },
            (4, _) => match (x < w / 2, y < h / 2) {
                    (true, true) => 0,
                    (true, false) => 1,
                    (false, true) => 2,
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
        sidebar_stack : Stack
    ) -> Self {
        let mapping_menus = Rc::new(RefCell::new(Vec::new()));
        let design_menu = build_design_menu(&builder, pl_view.clone());
        let scale_menus = build_scale_menus(&builder, pl_view.clone());
        let glade_def = Self::build_glade_def();
        let layout_path = Rc::new(RefCell::new(None));
        let plot_ev : EventBox = builder.get_object("plot_ev").unwrap();
        let sel_mapping = Rc::new(RefCell::new(String::new()));
        let plot_popover = PlotPopover::new(&builder);

        {
            let pl_view = pl_view.clone();
            let plot_popover = plot_popover.clone();
            plot_ev.connect_button_press_event(move |wid, ev| {
                let (x, y) = ev.get_position();
                let w = wid.get_allocation().width;
                let h = wid.get_allocation().height;
                if let Ok(mut pl) = pl_view.try_borrow_mut() {
                    let ix = Self::updated_active_area(&*pl, x as i32, y as i32, w, h);
                    println!("Active area: {}", ix);
                    pl.change_active_area(ix);
                    plot_popover.set_active_mapping(ix, None);
                } else {
                    println!("Failed acquiring mutable reference to plot view/selected mapping");
                }
                println!("Draw area touched at {:?}", (x, y));
                plot_popover.show_from_click(
                    &ev,
                    w,
                    h,
                    pl_view.borrow().group_split(),
                    pl_view.borrow().get_active_area()
                );
                glib::signal::Inhibit(true)
            });
        }
        let layout_window = LayoutWindow::new(
            builder.clone(),
            pl_view.clone(),
            mapping_menus.clone(),
            plot_popover.mapping_stack.clone(),
            layout_path.clone(),
            design_menu.clone(),
            scale_menus.clone()
        );
        let layout_toolbar = LayoutToolbar::build(
            builder.clone(),
            status_stack.clone(),
            sidebar_stack.clone(),
            pl_view.clone(),
            mapping_menus.clone(),
            plot_popover.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            layout_path.clone(),
            plot_toggle.clone(),
            glade_def.clone(),
            sel_mapping.clone()
        );
        layout_toolbar.connect_add_mapping_clicked(
            plot_popover.clone(),
            glade_def.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            pl_view.clone(),
            mapping_menus.clone(),
            status_stack.clone(),
            sel_mapping.clone()
        );
        layout_toolbar.connect_edit_mapping_clicked(
            plot_toggle.clone(),
            plot_popover.clone(),
            pl_view.clone(),
            tbl_nb.clone()
        );
        layout_toolbar.connect_remove_mapping_clicked(
            mapping_menus.clone(),
            pl_view.clone(),
            plot_popover.clone(),
            status_stack.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            layout_toolbar.mapping_popover.clone(),
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
            plot_popover.clone(),
            mapping_menus.clone(),
            design_menu.clone(),
            (scale_menus.0.clone(), scale_menus.1.clone()),
            plot_toggle,
            layout_window.clone(),
            layout_path.clone()
        );

        {
            let mapping_menus = mapping_menus.clone();
            let tbl_nb = tbl_nb.clone();
            let plot_popover = plot_popover.clone();
            let layout_toolbar = layout_toolbar.clone();
            plot_popover.tbl_btn.clone().connect_clicked(move |btn| {
                let mapping_ix = plot_popover.get_selected_mapping();
                layout_toolbar.update_selected_mapping(tbl_nb.clone(), mapping_menus.clone(), mapping_ix);
                table_toggle.set_active(true);
            });
        }
        if let Ok(pl_view) = pl_view.try_borrow() {
            if !pl_view.parent.get_realized() {
                pl_view.parent.realize();
            }
        } else {
            println!("Failed acquiring reference to plot view");
        }
        let ws = Self {
            design_menu,
            scale_menus,
            mapping_menus,
            sidebar_stack : sidebar_stack.clone(),
            pl_view : pl_view.clone(),
            layout_toolbar,
            glade_def,
            new_layout_btn,
            // load_layout_btn,
            // xml_load_dialog,
            layout_window,
            layout_path,
            plot_popover
        };
        ws.layout_window.connect_clear(&ws);
        ws
    }

    /// Add mapping from a type string description, attributing to its
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
        status_stack : StatusStack
    ) {
        println!("Adding mapping of type {}", mapping_type);
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
        let menu = MappingMenu::create(
            glade_def.clone(),
            Rc::new(RefCell::new(name)),
            mapping_type.to_string(),
            data_source.clone(),
            plot_view.clone(),
            None
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
                println!("Mapping appended");
            },
            Err(e) => { println!("{}", e); return; }
        }
    }

    pub fn clear_all_mappings(&self) -> Result<(), &'static str> {
        match (self.pl_view.try_borrow_mut(), self.mapping_menus.try_borrow()) {
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
        }
    }

    pub fn update_mapping_widgets(
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        glade_def : Rc<HashMap<String, String>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
    ) {
        let new_info = match plot_view.try_borrow() {
            Ok(pl_view) => pl_view.mapping_info(),
            Err(e) => { println!("{}", e); return; }
        };
        Self::clear_mappings(
            mapping_menus.clone(),
            plot_popover.mapping_stack.clone()
        ).expect("Error clearing mappings");
        for m_info in new_info.iter() {
            let menu = MappingMenu::create(
                glade_def.clone(),
                Rc::new(RefCell::new(m_info.0.clone())),
                m_info.1.clone(),
                data_source.clone(),
                plot_view.clone(),
                Some(m_info.2.clone()),
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
            }
        }
    }

    pub fn update_all_mappings(
        &self,
        t_env : &TableEnvironment,
        status_stack : StatusStack
    ) -> Result<(), &'static str> {
        let mut pl = self.pl_view.try_borrow_mut()
            .map_err(|_| "Could not get mutable reference to plot view")?;
        let menus = self.mapping_menus.try_borrow()
            .map_err(|_| "Could not get reference to mapping menus" )?;
        for m in menus.iter() {
            if let Err(e) = m.update_data(t_env, &mut pl) {
                status_stack.update(Status::SqlErr(format!("{}", e)));
                return Err("Error updating mappings");
            }
        }
        status_stack.update(Status::Ok);
        Ok(())
    }

    fn append_mapping_menu(
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

                plot_popover.add_mapping(&m);
                if let Ok(name) = m.mapping_name.try_borrow() {
                    pl.update(&mut UpdateContent::NewMapping(
                        name.clone(),
                        m.mapping_type.to_string())
                    );
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
    }

    pub fn clear(&self) {
        if let Err(e) = Self::clear_mappings(self.mapping_menus.clone(), self.plot_popover.mapping_stack.clone()) {
            println!("{}", e);
        }
        if let Ok(mut pl_view) = self.pl_view.try_borrow_mut() {
            pl_view.change_active_area(0);
            pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-unique.xml")));
        } else {
            println!("Failed to borrow mutable reference to plotview.");
        }
    }

    pub fn clear_mappings(
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
    }


}
