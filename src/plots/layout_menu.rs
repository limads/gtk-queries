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

#[derive(Clone)]
pub struct GroupToolbar {
    active_combo : ComboBoxText,
    toggle_unique : ToggleButton,
    toggle_horiz : ToggleButton,
    toggle_vert : ToggleButton,
    toggle_four : ToggleButton
}

impl GroupToolbar {

    fn set_sensitive_at_index(menus : &[MappingMenu], ix : usize) {
        for m in menus.iter() {
            if m.plot_ix == ix {
                m.tab_img.set_sensitive(true);
                m.set_sensitive(true);
            } else {
                m.tab_img.set_sensitive(false);
                m.set_sensitive(false);
            }
        }
    }

    fn new(
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        mapping_stack : Stack,
        _glade_def : Rc<HashMap<String, String>>,
        _tbl_nb : TableNotebook,
        _data_source : Rc<RefCell<TableEnvironment>>,
        _status_stack : StatusStack,
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu)
    ) -> GroupToolbar {
        let active_combo : ComboBoxText = builder.get_object("active_combo").unwrap();
        {
            let mapping_menus = mapping_menus.clone();
            // let _plot_notebook = plot_notebook.clone();
            let plot_view = plot_view.clone();
            active_combo.connect_changed(move |combo| {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    if let Ok(menus) = mapping_menus.try_borrow() {
                        match combo.get_active_text().as_ref().map(|s| s.as_str() ) {
                            Some("Top") => {
                                pl_view.change_active_area(0);
                                Self::set_sensitive_at_index(&menus[..], 0);
                            },
                            Some("Bottom") => {
                                pl_view.change_active_area(1);
                                Self::set_sensitive_at_index(&menus[..], 1);
                            },
                            Some("Left") => {
                                pl_view.change_active_area(0);
                                Self::set_sensitive_at_index(&menus[..], 0);
                            },
                            Some("Right") => {
                                pl_view.change_active_area(1);
                                Self::set_sensitive_at_index(&menus[..], 1);
                            },
                            Some("Top Left") => {
                                pl_view.change_active_area(0);
                                Self::set_sensitive_at_index(&menus[..], 0);
                            },
                            Some("Top Right") => {
                                pl_view.change_active_area(1);
                                Self::set_sensitive_at_index(&menus[..], 1);
                            },
                            Some("Bottom Left") => {
                                pl_view.change_active_area(2);
                                Self::set_sensitive_at_index(&menus[..], 2);
                            },
                            Some("Bottom Right") => {
                                pl_view.change_active_area(3);
                                Self::set_sensitive_at_index(&menus[..], 3);
                            },
                            _ => { }
                        }
                    } else {
                        println!("Unable to get reference to mapping menus");
                    }
                } else {
                    // TODO falling here
                    println!("Unable to retrieve mutable reference to plotview");
                }
                PlotSidebar::update_layout_widgets(
                    design_menu.clone(),
                    scale_menus.clone(),
                    plot_view.clone()
                );
            });
        }
        let toggle_unique : ToggleButton = builder.get_object("toggle_group_unique").unwrap();
        let toggle_horiz : ToggleButton = builder.get_object("toggle_group_horizontal").unwrap();
        let toggle_vert : ToggleButton = builder.get_object("toggle_group_vertical").unwrap();
        let toggle_four : ToggleButton = builder.get_object("toggle_group_four").unwrap();

        {
            let (toggle_horiz, toggle_vert, toggle_four) = (toggle_horiz.clone(), toggle_vert.clone(), toggle_four.clone());
            let active_combo = active_combo.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let mapping_stack = mapping_stack.clone();
            toggle_unique.connect_toggled(move |toggle_unique| {
                if toggle_unique.get_active() {
                    toggle_horiz.set_active(false);
                    toggle_vert.set_active(false);
                    toggle_four.set_active(false);
                    active_combo.remove_all();
                    active_combo.append(Some("Center"), "Center");
                    active_combo.set_active_id(Some("Center"));
                    active_combo.set_sensitive(false);
                    // plot_notebook.detach_tab(plot_notebook.)
                    PlotSidebar::clear_mappings(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                        //plot_notebook.clone()
                    ).expect("Error clearing mappings");
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-single.xml")));
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                }
            });
        }

        {
            let (toggle_unique, toggle_vert, toggle_four) = (toggle_unique.clone(), toggle_vert.clone(), toggle_four.clone());
            let active_combo = active_combo.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let mapping_stack = mapping_stack.clone();
            toggle_horiz.connect_toggled(move |toggle_horiz| {
                if toggle_horiz.get_active() {
                    toggle_unique.set_active(false);
                    toggle_vert.set_active(false);
                    toggle_four.set_active(false);
                    active_combo.remove_all();
                    active_combo.append(Some("Top"), "Top");
                    active_combo.append(Some("Bottom"), "Bottom");
                    active_combo.set_sensitive(true);
                    active_combo.set_active_id(Some("Top"));
                    PlotSidebar::clear_mappings(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                    ).expect("Error clearing mappings");
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-horiz.xml")));
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                }
            });
        }

        {
            let (toggle_unique, toggle_horiz, toggle_four) = (toggle_unique.clone(), toggle_horiz.clone(), toggle_four.clone());
            let active_combo = active_combo.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let mapping_stack = mapping_stack.clone();
            toggle_vert.connect_toggled(move |toggle_vert| {
                if toggle_vert.get_active() {
                    toggle_unique.set_active(false);
                    toggle_horiz.set_active(false);
                    toggle_four.set_active(false);
                    active_combo.remove_all();
                    active_combo.append(Some("Left"), "Left");
                    active_combo.append(Some("Right"), "Right");
                    active_combo.set_sensitive(true);
                    active_combo.set_active_id(Some("Left"));
                    PlotSidebar::clear_mappings(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                    ).expect("Error clearing mappings");
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-vert.xml")));
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                }
            });
        }

        {
            let (toggle_unique, toggle_horiz, toggle_vert) =
                (toggle_unique.clone(), toggle_horiz.clone(), toggle_vert.clone());
            let active_combo = active_combo.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let mapping_stack = mapping_stack.clone();
            toggle_four.connect_toggled(move |toggle_four| {
                if toggle_four.get_active() {
                    toggle_unique.set_active(false);
                    toggle_horiz.set_active(false);
                    toggle_vert.set_active(false);
                    active_combo.remove_all();
                    active_combo.append(Some("Top Left"), "Top Left");
                    active_combo.append(Some("Top Right"), "Top Right");
                    active_combo.append(Some("Bottom Left"), "Bottom Left");
                    active_combo.append(Some("Bottom Right"), "Bottom Right");
                    active_combo.set_sensitive(true);
                    active_combo.set_active_id(Some("Top Left"));
                    PlotSidebar::clear_mappings(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                    ).expect("Error clearing mappings");
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-four.xml")));
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                }
            });
        }
        GroupToolbar {
            active_combo,
            toggle_unique,
            toggle_horiz,
            toggle_vert,
            toggle_four
        }
    }

    pub fn reset(&self, split : GroupSplit) {
        match split {
            GroupSplit::None => { self.toggle_unique.toggled(); }
            GroupSplit::Horizontal => { self.toggle_horiz.toggled(); }
            GroupSplit::Vertical => { self.toggle_vert.toggled(); }
            GroupSplit::Both => { self.toggle_four.toggled(); }
        }
    }
}

/// PlotsSidebar holds the information of the used mappings
#[derive(Clone)]
pub struct PlotSidebar {
    pub design_menu : DesignMenu,
    pub scale_menus : (ScaleMenu, ScaleMenu),
    pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    //pub notebook : Notebook,
    pub sidebar_stack : Stack,
    pub pl_view : Rc<RefCell<PlotView>>,
    pub plot_popover : PlotPopover,
    pub layout_toolbar : LayoutToolbar,
    new_layout_btn : Button,
    load_layout_btn : Button,
    glade_def : Rc<HashMap<String, String>>,
    xml_load_dialog : FileChooserDialog,
    group_toolbar : GroupToolbar,
    layout_path : Rc<RefCell<Option<String>>>
}

impl PlotSidebar {

    pub fn set_active(&self, state : bool) {
        // self.new_layout_btn.set_sensitive(state);
        // self.load_layout_btn.set_sensitive(state);
        /*if let Err(e) = self.layout_toolbar.set_add_mapping_sensitive(0) {
            println!("{}", e);
        }
        if let Err(e) = self.layout_toolbar.set_edit_mapping_sensitive(0) {
            println!("{}" ,e);
        }*/
        if state == false {
            self.xml_load_dialog.unselect_all();
        }
        if let Some(true) = self.sidebar_stack.get_visible_child_name()
            .map(|n| n.as_str() == "layout" ) {
            self.sidebar_stack.set_visible_child_name("empty");
        } else {
            self.sidebar_stack.set_visible_child_name("database");
        }
    }

    pub fn layout_loaded(&self) -> bool {
        /*let sel_name = self.sidebar_stack.get_visible_child_name()
            .map(|n| n.to_string()).unwrap_or(String::from("empty"));
        match &sel_name[..] {
            "layout" => true,
            _ => false
        }*/
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
                    "assets/plot_layout/layout-single.xml"
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
        (x, y, w, h)
    }

    fn updated_active_area(pl_view : &PlotView, x : i32, y : i32, w : i32, h : i32) -> usize {
        match (pl_view.plot_group.size(), pl_view.plot_group.group_split()) {
            (1, _) => 0,
            (2, GroupSplit::Horizontal) => if y < h / 2 { 0 } else { 1 },
            (2, GroupSplit::Vertical) => if x < w / 2 { 0 } else { 1 },
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
        //let plot_notebook : Notebook =
        //    builder.get_object("plot_notebook").unwrap();
        let scale_menus = build_scale_menus(&builder, pl_view.clone());
        //let sidebar_stack : Stack = builder.get_object("sidebar_stack").unwrap();
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
                plot_popover.show_from_click(&ev, w, h);
                glib::signal::Inhibit(true)
            });
        }
        let group_toolbar = GroupToolbar::new(
            builder.clone(),
            pl_view.clone(),
            mapping_menus.clone(),
            plot_popover.mapping_stack.clone(),
            glade_def.clone(),
            tbl_nb.clone(),
            table_env.clone(),
            status_stack.clone(),
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
        let (load_layout_btn, xml_load_dialog) = Self::build_layout_load_button(
            glade_def.clone(),
            builder.clone(),
            pl_view.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            status_stack.clone(),
            layout_toolbar.clear_layout_btn.clone(),
            plot_popover.clone(),
            mapping_menus.clone(),
            design_menu.clone(),
            (scale_menus.0.clone(), scale_menus.1.clone()),
            plot_toggle,
            sidebar_stack.clone(),
            group_toolbar.clone(),
            layout_path.clone()
        );
        {

            let mapping_menus = mapping_menus.clone();
            let tbl_nb = tbl_nb.clone();
            let plot_popover = plot_popover.clone();
            let layout_toolbar = layout_toolbar.clone();
            //let sel_mapping = plot_popover.sel_mapping.clone();
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
        Self {
            design_menu,
            scale_menus,
            mapping_menus,
            //mapping_popover,
            //notebook : plot_notebook.clone(),
            sidebar_stack : sidebar_stack.clone(),
            pl_view : pl_view.clone(),
            layout_toolbar,
            glade_def,
            new_layout_btn,
            load_layout_btn,
            xml_load_dialog,
            group_toolbar,
            layout_path,
            plot_popover
        }
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
            None,
            //mapping_menus.clone()
        );
        match menu {
            Ok(m) => {
                Self::append_mapping_menu(
                    m,
                    mapping_menus.clone(),
                    plot_popover.clone(),
                    status_stack.clone(),
                    //plot_notebook.clone(),
                    // mapping_stack.clone(),
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

    fn update_mapping_widgets(
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        // plot_notebook : Notebook,
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
            //plot_notebook.clone()
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

    fn build_layout_load_button(
        glade_def : Rc<HashMap<String, String>>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        //sidebar : PlotSidebar,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        layout_clear_btn : ToolButton,
        //plot_notebook : Notebook,
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_toggle : ToggleButton,
        sidebar_stack : Stack,
        group_toolbar : GroupToolbar,
        layout_path : Rc<RefCell<Option<String>>>
    ) -> (Button, FileChooserDialog) {
        let xml_load_dialog : FileChooserDialog =
            builder.get_object("xml_load_dialog").unwrap();
        let load_btn : Button=
            builder.get_object("load_layout_btn").unwrap();
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
            });
        }

        {
            xml_load_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            let update_ok = match plot_view.try_borrow_mut() {
                                Ok(mut pl) => {
                                    let string_path : String = path.to_str().unwrap_or("").into();
                                    match pl.plot_group.load_layout(string_path.clone()) {
                                        Ok(_) => {
                                            group_toolbar.reset(pl.group_split());
                                            *(layout_path.borrow_mut()) = Some(string_path);
                                            true
                                        },
                                        Err(e) => { println!("{}", e); false }
                                    }
                                },
                                Err(_) => { println!("Could not get mutable reference to Plot widget"); false }
                            };
                            if update_ok {
                                Self::update_mapping_widgets(
                                    plot_view.clone(),
                                    mapping_menus.clone(),
                                    //plot_notebook.clone(),
                                    plot_popover.clone(),
                                    glade_def.clone(),
                                    data_source.clone(),
                                    tbl_nb.clone(),
                                    status_stack.clone()
                                );
                                Self::update_layout_widgets(
                                    design_menu.clone(),
                                    scale_menus.clone(),
                                    plot_view.clone()
                                );
                                //plot_notebook.show_all();
                                status_stack.try_show_alt();
                                plot_toggle.set_active(true);
                                sidebar_stack.set_visible_child_name("layout");
                                layout_clear_btn.set_sensitive(true);
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
        (load_btn, xml_load_dialog)
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
                //m.update_available_cols(source.col_names(), &pl);
                match pos {
                    Some(p) => mappings.insert(p, m.clone()),
                    None => mappings.push(m.clone())
                }

                plot_popover.add_mapping(&m);

                /*notebook.add(&m.get_parent());
                notebook.set_tab_label(&m.get_parent(), Some(&m.tab_img));
                let npages = notebook.get_children().len() as i32;
                notebook.set_property_page(npages-1);
                notebook.show_all();*/

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
            pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-single.xml")));
        } else {
            println!("Failed to borrow mutable reference to plotview.");
        }
    }

    fn clear_mappings(
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

    fn update_layout_widgets(
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_view : Rc<RefCell<PlotView>>
    ) {
        match plot_view.try_borrow_mut() {
            Ok(pl) => {
                design_menu.update(pl.plot_group.design_info());
                scale_menus.0.update(pl.current_scale_info("x"));
                scale_menus.1.update(pl.current_scale_info("y"));
            },
            _ => {
                // TODO panicking here when loading layout.
                panic!("Could not fetch plotview reference to update layout");
            }
        }
    }


}

/*
/*// LayoutMenu encapsulate the logic of the buttons at the bottom-left
// that allows changing the plot layout and mappings.
#[derive(Clone)]
pub struct LayoutMenu {

    new_layout_btn : Button,
    // add_mapping_btn : ToolButton,
    // manage_btn : Button,
    // remove_mapping_btn : ToolButton,
    sidebar_stack : Stack,
    glade_def : Rc<String>,

    //manage_mapping_popover : Popover
}*/

/*fn load_text_content(path : PathBuf)
-> Option<String> {
    if let Ok(mut f) = File::open(path) {
        let mut content = String::new();
        let has_read = f.read_to_string(&mut content);
        if has_read.is_ok() {
            return Some(content);
        } else {
            None
        }
    } else {
        None
    }
}*/

//impl LayoutMenu {

    /*fn build_layout_load_button(
        glade_def : Rc<String>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        sidebar : PlotSidebar,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        layout_clear_btn : ToolButton
    ) -> Button {
        let xml_load_dialog : FileChooserDialog =
            builder.get_object("xml_load_dialog").unwrap();
        let load_btn : Button=
            builder.get_object("load_layout_btn").unwrap();
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
            });
        }

        {
            xml_load_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            //let path = f.get_path().unwrap_or(PathBuf::new());
                            //println!("{:?}", path);
                            //if let Some(path) = f {
                            let new_mapping_info = match plot_view.try_borrow_mut() {
                                Ok(mut pl) => {
                                    match pl.plot_area.load_layout(path.to_str().unwrap_or("").into()) {
                                        Ok(_) => Some(pl.plot_area.mapping_info()),
                                        Err(e) => { println!("{}", e); None }
                                    }
                                },
                                Err(_) => { println!("Could not get reference to Plot widget"); None }
                            };
                            if let Some(new_info) = new_mapping_info {
                                Self::clear_mappings(
                                    sidebar.mapping_menus.clone(),
                                    sidebar.notebook.clone()
                                ).expect("Error clearing mappings");
                                Self::update_layout_widgets(
                                    sidebar.clone(),
                                    plot_view.clone()
                                );
                                layout_clear_btn.set_sensitive(true);
                                for m_info in new_info.iter() {
                                    let menu = Self::create_new_mapping_menu(
                                        glade_def.clone(),
                                        //builder.clone(),
                                        Rc::new(RefCell::new(m_info.0.clone())),
                                        m_info.1.clone(),
                                        data_source.clone(),
                                        plot_view.clone(),
                                        Some(m_info.2.clone()),
                                        sidebar.clone()
                                    );
                                    match menu {
                                        Ok(m) => {
                                            Self::append_mapping_menu(
                                                m,
                                                sidebar.mapping_menus.clone(),
                                                sidebar.notebook.clone(),
                                                plot_view.clone(),
                                                data_source.clone(),
                                                tbl_nb.clone(),
                                                None
                                            );
                                        },
                                        Err(e) => { println!("{}", e); return; }
                                    }
                                }
                                sidebar.notebook.show_all();
                                status_stack.try_show_alt();
                                // sidebar.sidebar_stack.set_visible_child_name("layout");
                                // println!("{:?}", mappings);
                            } else {
                                println!("No info to update");
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
        load_btn
    }*/

    /*fn selected_mapping_radio(scatter_radio : &RadioButton) -> Option<String> {
        for radio in scatter_radio.get_group() {
            if radio.get_active() {
                if let Some(name) = WidgetExt::get_widget_name(&radio) {
                    return Some(name.as_str().to_string());
                }
            }
        }
        None
    }

    fn set_mapping_radio(scatter_radio : &RadioButton, curr_type : String) {
        for radio in scatter_radio.get_group() {
            if let Some(name) = WidgetExt::get_widget_name(&radio) {
                if name == &curr_type[..] {
                    radio.set_active(true);
                    return;
                }
            }
        }
        println!("Radio not found for informed type");
    }*/

    /*pub fn new_from_builder(
        builder : &Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        sidebar : PlotSidebar
    ) -> Self {





        Self {
            load_layout_btn,
            add_mapping_btn,
            new_layout_btn,
            remove_mapping_btn,
            sidebar_stack,
            glade_def,
            mapping_btns
            //manage_mapping_popover
        }
    }

}*/

*/
