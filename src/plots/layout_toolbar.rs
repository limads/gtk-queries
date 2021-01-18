use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::environment::TableEnvironment;
use crate::plots::plotview::GroupSplit;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
// use std::fs::File;
// use std::io::Read;
// use super::design_menu::*;
// use super::scale_menu::*;
// use super::layout_window::*;
use super::mapping_menu::*;
// use super::plot_popover::*;
use std::collections::HashMap;
// use crate::utils;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
// use std::default::Default;
use crate::plots::plot_workspace::PlotWorkspace;
use super::layout_window::MappingTree;
use super::mapping_menu::DataSource;
use crate::plots::layout_window::LayoutWindow;

#[derive(Clone, Debug)]
pub struct SelectedMapping {

    ty : String,

    pl_ix : usize,

    global_ix : usize,

    local_ix : usize
}

#[derive(Clone, Debug)]
pub enum SelectionStatus {

    // Carries the new plot index and new mapping type
    New(usize, String),

    // Carries the selected plot index, global mapping index
    Single(SelectedMapping),

    // Multiple mappings are selected and no useful information can be retrieved
    Multiple(Vec<SelectedMapping>),

    // No mapping is selected
    None
}

/// LayoutToolbar encapsulates the logic for the popover when the user right-click
/// some table columns to add, edit or remove mappings.
#[derive(Debug, Clone)]
pub struct LayoutToolbar {
    pub add_mapping_btn : ToolButton,
    pub edit_mapping_btn : ToolButton,
    pub clear_layout_btn : ToolButton,
    pub remove_mapping_btn : ToolButton,
    pub mapping_popover : Popover,
    pub mapping_btns : HashMap<String, ToggleToolButton>,
    pub group_toolbar : GroupToolbar,
    selection : Rc<RefCell<SelectionStatus>>
}

/// Represents which toogle will be used to decide the plot layout when inserting/editing.
#[derive(Clone, Debug)]
pub struct GroupToolbar {
    pub layout_stack : Stack,

    // (index to layout_stack position, if any at this position is active)
    active : Rc<RefCell<(usize, Option<usize>)>>,
    sensitive : Rc<RefCell<bool>>,
    toggles : Vec<Vec<ToggleToolButton>>
}

impl GroupToolbar {

    fn build_group_toolbar(
        builder : &Builder,
        prefix : &str,
        layout_stack : &Stack,
        plot_view : &Rc<RefCell<PlotView>>,
        // plot_popover : &PlotPopover,
        selection : &Rc<RefCell<SelectionStatus>>,
        // mapping_menus : &Rc<RefCell<Vec<MappingMenu>>>,
        active : &Rc<RefCell<(usize, Option<usize>)>>,
    ) -> Vec<ToggleToolButton> {
        let toolbar : Toolbar = builder.get_object(&format!("{}_layout_toolbar", prefix)).unwrap();
        let n_opts = match prefix {
            "unique" => 1,
            "vert" | "horiz" => 2,
            prefix => if prefix.starts_with("three") {
                3
            } else {
                4
            }
        };
        let mut btns = Vec::new();
        for i in 1..(n_opts+1) {
            let name = format!("{}-{}", prefix, i);
            let file_path = format!("assets/icons/layout/{}", name) +  ".svg";
            let img = Image::from_file(&file_path[..]);
            let btn : ToggleToolButton = ToggleToolButton::new();
            btn.set_icon_widget(Some(&img));
            toolbar.insert(&btn, i-1);
            btns.push(btn);
        }
        for (i, btn) in btns.iter().enumerate() {
            let btns_c = btns.clone();
            let plot_view = plot_view.clone();
            // let plot_popover = plot_popover.clone();
            let selection = selection.clone();
            // let mapping_menus = mapping_menus.clone();
            let active = active.clone();
            btn.connect_toggled(move |btn| {
                if btn.get_active() {
                    for (j, alt_btn) in btns_c.iter().enumerate() {
                        if i != j {
                            alt_btn.set_active(false);
                        }
                    }
                    // let selection = *selection.borrow_mut();
                    // println!("Current selection: {:?}", selection);
                    if let Ok(mut active) = active.try_borrow_mut() {
                        active.1 = Some(i);
                    } else {
                        println!("Unable to borrow plot selection");
                        return;
                    }
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.set_active_area(i as usize);
                        // println!("Current active area: {:?}", *active.borrow());
                    } else {
                        println!("Unable to borrow plot view mutably");
                    }

                    /*match *selection.borrow_mut() {
                        SelectionStatus::New(ref mut ix, _) => {
                            // *ix = i;
                        },
                        SelectionStatus::Single(_) => {
                            // Should be insensitive here
                            /*let sel = if let Some(sel) = plot_popover.get_selected_mapping() {
                                sel
                            } else {
                                println!("No current mapping selected");
                                return;
                            };
                            println!("Selected mapping: {}", sel);
                            match (mapping_menus.try_borrow_mut(), plot_view.try_borrow_mut()) {
                                (Ok(mut menus), Ok(mut pl_view)) => {
                                    if let Some(mut m) = menus.get_mut(sel) {
                                        let old_ix = m.plot_ix;
                                        let old_name = m.mapping_name.borrow().clone();
                                        let new_name = format!("{}", pl_view.plot_group.n_mappings()[i]);
                                        m.plot_ix = i;
                                        *m.mapping_name.borrow_mut() = new_name;
                                        pl_view.update(&mut UpdateContent::ReassignPlot((old_ix, old_name, i)));
                                        plot_popover.replace_mapping(old_ix, sel, i);
                                    } else {
                                        println!("Unable to get mapping menu");
                                    }
                                    println!("Mappings after insertion: {:?}", menus);
                                },
                                _ => {
                                    println!("Could not get mutable reference to mapping menus");
                                }
                            }*/
                        },
                        SelectionStatus::None | SelectionStatus::Multiple => {
                            // Should be insensitive here.
                        }
                    }*/
                } else {

                }
            });
        }
        btns
    }

    pub fn set_sensitive(&self, sensitive : bool) {
        // The unique plot layout is always insensitive
        if self.active.borrow().0 != 0 {
            for btn_set in self.toggles.iter() {
                for btn in btn_set.iter() {
                    btn.set_sensitive(sensitive);
                }
            }
            *(self.sensitive.borrow_mut()) = sensitive;
        } else {
            for btn_set in self.toggles.iter() {
                for btn in btn_set.iter() {
                    btn.set_sensitive(false);
                }
            }
        }
    }

    pub fn get_sensitive(&self) -> bool {
        *(self.sensitive.borrow())
    }

    pub fn get_group_ix(&self) -> usize {
        self.active.borrow().0
    }

    pub fn get_active_area(&self) -> Option<usize> {
        self.active.borrow().1
    }

    pub fn set_inactive(&self) {
        // The unique plot layout is always active
        if self.active.borrow().0 != 0 {
            for btn_set in self.toggles.iter() {
                for btn in btn_set.iter() {
                    btn.set_active(false);
                }
            }
            if let Ok(mut active) = self.active.try_borrow_mut() {
                active.1 = None;
            } else {
                println!("Failed getting mutable reference to active status");
            }
        }
    }

    pub fn set_active(&self, stack_pos : usize, toggle_pos : usize) {
        self.set_inactive();
        let name = match stack_pos {
            0 => "unique",
            1 => "vert",
            2 => "horiz",
            3 => "three-left",
            4 => "three-top",
            5 => "three-right",
            6 => "three-bottom",
            7 => "four",
            _ => panic!("Invalid layout stack pos")
        };
        self.layout_stack.set_visible_child_name(name);
        self.layout_stack.show_all();
        let toggles = self.toggles[stack_pos].iter();
        if let Ok(mut active) = self.active.try_borrow_mut() {
            active.0 = stack_pos;
            active.1 = Some(toggle_pos);
        } else {
            println!("Unable to borrow toggle pos mutably");
            return;
        }
        for (i, toggle) in toggles.enumerate() {
            if !toggle.get_sensitive() {
                toggle.set_sensitive(true);
            }
            if i == toggle_pos {
                toggle.set_active(true);
            } else {
                toggle.set_active(false);
            }
        }
        if self.active.borrow().0 == 0 {
            self.set_sensitive(false);
        }
    }

    pub fn set_active_default(&self, layout : Option<GroupSplit>) {
        if let Some(layout) = layout {
            let new_layout_ix = match layout {
                GroupSplit::Unique => 0,
                GroupSplit::Vertical => 1,
                GroupSplit::Horizontal => 2,
                GroupSplit::ThreeLeft => 3,
                GroupSplit::ThreeTop => 4,
                GroupSplit::ThreeRight =>5,
                GroupSplit::ThreeBottom => 6,
                GroupSplit::Four => 7
            };
            self.set_active(new_layout_ix, 0);
        } else {
            let curr_group = self.get_group_ix();
            self.set_active(curr_group, 0);
        }
    }

    /// Kepp the same stack position, but switch the active plot
    pub fn switch_active_plot(&self, plot_ix : usize) {
        self.set_active(self.get_group_ix(), plot_ix);
    }

    pub fn build(
        builder : &Builder,
        plot_view : &Rc<RefCell<PlotView>>,
        // plot_popover : &PlotPopover,
        selection : &Rc<RefCell<SelectionStatus>>,
        // mapping_menus : &Rc<RefCell<Vec<MappingMenu>>>
    ) -> Self {
        let layout_stack : Stack = builder.get_object("layout_stack").unwrap();
        let mut toggles = Vec::new();
        let active = Rc::new(RefCell::new((0, None)));
        let prefixes = ["unique", "vert", "horiz", "three-left",
            "three-top", "three-right", "three-bottom", "four"];
        for prefix in prefixes.iter() {
            toggles.push(Self::build_group_toolbar(
                &builder,
                prefix,
                &layout_stack,
                &plot_view,
                // &plot_popover,
                &selection,
                // &mapping_menus,
                &active
            ));
        }
        let group_toolbar = Self {
            layout_stack,
            toggles,
            sensitive : Rc::new(RefCell::new(true)),
            active
        };
        group_toolbar.set_active(0, 0);
        group_toolbar
    }

    pub fn any_selected(&self) -> bool {
        self.active.borrow().1.is_some()
    }

}

impl LayoutToolbar {

    pub fn build(
        builder : Builder,
        status_stack : StatusStack,
        sidebar_stack : Stack,
        plot_view : Rc<RefCell<PlotView>>,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        // plot_popover : PlotPopover,
        sources : Rc<RefCell<Vec<DataSource>>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        layout_path : Rc<RefCell<Option<String>>>,
        plot_toggle : ToggleButton,
        glade_def : Rc<HashMap<String, String>>,
        sel_mapping : Rc<RefCell<String>>
    ) -> Self {
        let layout_toolbar : Toolbar = builder
            .get_object("layout_toolbar").unwrap();
        let selection = Rc::new(RefCell::new(SelectionStatus::None));
        let group_toolbar = GroupToolbar::build(
            &builder,
            &plot_view,
            // &plot_popover,
            &selection,
            // &mapping_menus
        );
        let img_add = Image::from_icon_name(Some("list-add-symbolic"), IconSize::SmallToolbar);
        let img_edit = Image::from_icon_name(Some("document-edit-symbolic"), IconSize::SmallToolbar);
        let img_remove = Image::from_icon_name(Some("list-remove-symbolic"), IconSize::SmallToolbar);
        let img_clear = Image::from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let clear_layout_btn : ToolButton = ToolButton::new(Some(&img_clear), None);
        let add_mapping_btn : ToolButton = ToolButton::new(Some(&img_add), None);
        let edit_mapping_btn : ToolButton = ToolButton::new(Some(&img_edit), None);
        let remove_mapping_btn : ToolButton = ToolButton::new(Some(&img_remove), None);
        // TODO verify if there isn't already at least two columns selected. If there is, do not set
        // add sensititve to false.
        remove_mapping_btn.set_sensitive(false);
        add_mapping_btn.set_sensitive(false);
        edit_mapping_btn.set_sensitive(false);
        clear_layout_btn.set_sensitive(false);
        layout_toolbar.insert(&clear_layout_btn, 0);
        layout_toolbar.insert(&add_mapping_btn, 1);
        layout_toolbar.insert(&edit_mapping_btn, 2);
        layout_toolbar.insert(&remove_mapping_btn, 3);
        layout_toolbar.show_all();
        {
            let sidebar_stack = sidebar_stack.clone();
            let plot_view = plot_view.clone();
            // let mapping_menus = mapping_menus.clone();
            let status_stack = status_stack.clone();
            let layout_path = layout_path.clone();
            let group_toolbar = group_toolbar.clone();
            clear_layout_btn.connect_clicked(move |btn| {
                //TODO toggle group toolbar to single
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-unique.xml")));
                    sidebar_stack.set_visible_child_name("empty");
                    group_toolbar.set_active(0, 0);
                    *(layout_path.borrow_mut()) = None;
                    status_stack.show_curr_status();
                    /*if let Ok(mut mappings) = mapping_menus.try_borrow_mut() {
                        mappings.clear();
                    } else {
                        println!("Error retrieving mapping menus");
                    }*/
                }
                btn.set_sensitive(false);

            });
        }

        /*{
            let mapping_menus = mapping_menus.clone();
            let plot_view = plot_view.clone();
            //let mapping_stack = mapping_stack.clone();
            let table_env = table_env.clone();
            let status_stack = status_stack.clone();
            let tbl_nb = tbl_nb.clone();
            /*edit_mapping_btn.connect_clicked(move |_| {
                let selected_cols = tbl_nb.full_selected_cols();
                plot_toggle.set_active(true);

                // Substitute by stack logic instead.
                /*let page = notebook.get_property_page() as usize;
                if page <= 2 || selected_cols.len() == 0 {
                    return;
                }
                match (plot_view.try_borrow_mut(), mapping_menus.try_borrow(), table_env.try_borrow()) {
                    (Ok(mut pl_view), Ok(menus), Ok(t_env)) => {
                        if let Some(m) = menus.get(page - 3) {
                            if let Err(e) = m.reassign_data(selected_cols, &t_env, &mut pl_view) {
                                status_stack.update(Status::SqlErr(e.to_string()));
                            } else {
                                pl_view.redraw();
                            }
                        } else {
                            println!("No mapping at index {}", page - 3);
                        }
                    },
                    _ => {
                        println!("Unable to retrieve reference to menus or plotview");
                    }
                }*/
            });*/
        }*/

        let (mapping_btns, mapping_popover) = LayoutToolbar::build_add_mapping_toggles(
            builder.clone(),
            add_mapping_btn.clone(),
            edit_mapping_btn.clone(),
            remove_mapping_btn.clone(),
            table_env.clone(),
            plot_view.clone(),
            tbl_nb.clone(),
            glade_def.clone(),
            sources.clone(),
            // mapping_menus.clone(),
            // plot_popover.clone(),
            status_stack.clone(),
            sel_mapping.clone(),
            selection.clone(),
            group_toolbar.clone()
        );

        {
            let mapping_btns = mapping_btns.clone();
            mapping_popover.connect_closed(move |popover| {
                for (_, btn) in mapping_btns.iter() {
                    btn.set_active(false);
                }
            });
        }
        let layout_toolbar = Self {
            add_mapping_btn,
            edit_mapping_btn,
            clear_layout_btn,
            remove_mapping_btn,
            mapping_btns,
            mapping_popover,
            group_toolbar : group_toolbar.clone(),
            selection
        };

        {
            let tbl_nb = tbl_nb.clone();
            let layout_toolbar = layout_toolbar.clone();
            // let plot_popover = plot_popover.clone();
            // let mapping_menus = mapping_menus.clone();
            let group_toolbar = group_toolbar.clone();
            layout_toolbar.mapping_popover.clone().connect_show(move |wid| {

                // This is necessary because the table set_sensitive=false propagates
                // to the popover when new data arrives
                if !wid.get_sensitive() {
                    wid.set_sensitive(true);
                }

                let (tbl_ix, selected_ix) = tbl_nb.selected_table_and_cols()
                    .unwrap_or((0, Vec::new()));
                layout_toolbar.update_mapping_status(
                    sources.clone(),
                    // &plot_popover,
                    &selected_ix[..],
                    tbl_ix
                );
            });
        }

        layout_toolbar
    }

    pub fn connect_add_mapping_clicked(
        &self,
        // plot_popover : PlotPopover,
        glade_def : Rc<HashMap<String, String>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        plot_view : Rc<RefCell<PlotView>>,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        status_stack : StatusStack,
        sel_mapping : Rc<RefCell<String>>,
        mapping_tree : MappingTree,
        sources : Rc<RefCell<Vec<DataSource>>>
    ) {
        let edit_mapping_btn = self.edit_mapping_btn.clone();
        let remove_mapping_btn = self.remove_mapping_btn.clone();
        let mapping_btns = self.mapping_btns.clone();
        let group_toolbar = self.group_toolbar.clone();
        let selection = self.selection.clone();
        self.add_mapping_btn.connect_clicked(move|btn| {
            println!("Group toolbar active area = {:?}", group_toolbar.get_active_area());
            let active_area = if let Some(active) = group_toolbar.get_active_area() {
                active
            } else {
                println!("No current active area to add mapping");
                return;
            };
            println!("Adding to active area: {}", active_area);
            
            let ty = sel_mapping.borrow();
            
            /* PlotWorkspace::add_mapping_from_type(
                glade_def.clone(),
                &m[..],
                table_env.clone(),
                tbl_nb.clone(),
                plot_view.clone(),
                mapping_menus.clone(),
                plot_popover.clone(),
                status_stack.clone(),
                active_area
            );*/
            
            if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                pl_view.set_active_area(active_area);
                let name = pl_view.mapping_info().len().to_string();
                let mut source : DataSource = Default::default();
                source.name = name.to_string();
                source.ty = ty.to_string();
                source.plot_ix = active_area;
                source.hist_ix = table_env.borrow().current_hist_index();
                pl_view.update(&mut UpdateContent::NewMapping(
                    name.clone(),
                    ty.to_string(),
                    active_area
                ));
                let info = pl_view.mapping_info();
                let (_, _, last_inserted_info) = info.last().clone().unwrap();
                mapping_tree.append_mapping(active_area, &ty[..], last_inserted_info);
                if let Ok(t_env) = table_env.try_borrow() {
                    PlotWorkspace::update_source(&mut source, tbl_nb.full_selected_cols(), &t_env)
                        .map_err(|e| format!("{}", e) );
                    PlotWorkspace::update_data(&source, &t_env, &mut pl_view)
                        .map_err(|e| format!("{}", e) );
                    sources.borrow_mut().push(source);
                }
                
                // As soon as a new plot is added, it becomes the unique selection.
                if let Ok(mut sel) = selection.try_borrow_mut() {
                    *sel = SelectionStatus::Single(SelectedMapping {
                        ty : ty.to_string(),
                        pl_ix : active_area,
                        local_ix : sources.borrow().len() - 1,
                        global_ix : info.len() - 1
                    });
                } else {
                    println!("Unable to borrow selection");
                }
            } else {
                println!("Unable to get reference to plot view");
                return;
            };
            
            // TODO Call update_source(.) then call update_data(.)
            
            /*println!("Mapping appended: {:?}", m);
            if with_data {
                if let Err(e) = m.reassign_data(tbl_nb.full_selected_cols(), &t_env, &mut pl) {
                    status_stack.update(Status::SqlErr(format!("{}", e)));
                    return;
                }
            } else {
                if let Err(e) = m.clear_data(&mut pl) {
                    println!("{}", e);
                }
            }*/
            
            btn.set_sensitive(false);
            edit_mapping_btn.set_sensitive(true);
            remove_mapping_btn.set_sensitive(true);
            group_toolbar.set_sensitive(false);
            // plot_popover.update_nav_sensitive();
            // plot_popover.update_stack();
        });
    }

    pub fn connect_edit_mapping_clicked(
        &self,
        plot_toggle : ToggleButton,
        // plot_popover : PlotPopover,
        layout_window : LayoutWindow,
        pl_view : Rc<RefCell<PlotView>>,
        tbl_nb : TableNotebook
    ) {
        let mapping_popover = self.mapping_popover.clone();
        let selection = self.selection.clone();
        self.edit_mapping_btn.connect_clicked(move |btn| {
            plot_toggle.set_active(true);
            mapping_popover.hide();
            tbl_nb.unselect_all_tables();
            if layout_window.win.get_visible() {
                layout_window.win.grab_focus();
            } else {
                layout_window.win.show();
            }
            layout_window.layout_stack.set_visible_child_name("mapping");
            layout_window.mapping_tree.tree_view.expand_all();
            if let Ok(sel) = selection.try_borrow() {
                println!("Selection: {:?}", sel);
                if let SelectionStatus::Single(sel) = sel.clone() {
                    layout_window.mapping_tree.set_selected(sel.pl_ix, sel.local_ix);
                } else {
                    println!("Selection status is not single");
                }
            } else {
                println!("Unable to borrow selection status");
            }
            
            // let (x, y, w, h) = PlotWorkspace::get_active_coords(&pl_view.borrow());
            // plot_popover.show_at(x, y, w, h);
            // Disable most recent toggle
            // mapping_btns.iter()
            //    .filter(|(name, btn)| &name[..] == &m[..] )
            //    .for_each(|(name, btn)| btn.set_active(false) );
        });
    }

    pub fn connect_remove_mapping_clicked(
        &self,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>,
        // plot_popover : PlotPopover,
        status_stack : StatusStack,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        mapping_popover : Popover,
        mapping_tree : MappingTree,
        sources : Rc<RefCell<Vec<DataSource>>>,
    ) {
        /*println!("Remove mapping clicked");
        self.mapping_popover.hide();*/
        let selection = self.selection.clone();
        let add_mapping_btn = self.add_mapping_btn.clone();
        let edit_mapping_btn = self.edit_mapping_btn.clone();
        let mapping_btns = self.mapping_btns.clone();
        self.remove_mapping_btn.connect_clicked(move |remove_btn| {
            if let Ok(sel) = selection.try_borrow() {
                if let SelectionStatus::Single(sel) = sel.clone() {
                    mapping_tree.remove_mapping(sel.pl_ix, sel.local_ix);
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.update(&mut UpdateContent::RemoveMapping(
                            sel.pl_ix,
                            sel.local_ix.to_string()
                        ));
                        
                        println!("Remaining mappipings: {:?}", pl_view.mapping_info());
                    } else {
                        println!("Unable to borrow plot view");
                    }
                    
                    tbl_nb.unselect_all_tables();
                    add_mapping_btn.set_sensitive(false);
                    edit_mapping_btn.set_sensitive(false);
                    remove_btn.set_sensitive(false);
                    for (_, btn) in mapping_btns.iter() {
                        if btn.get_active() {
                            btn.set_active(false);
                        }
                        btn.set_sensitive(false);
                    }
                    
                    if let Ok(mut sources) = sources.try_borrow_mut() {
                        let rem_pos = sources.iter().position(|source| {
                            let is_plot = source.plot_ix == sel.pl_ix; 
                            let is_mapping = source.name.parse::<usize>().unwrap() == sel.local_ix;
                            is_plot && is_mapping
                        }).unwrap();
                        sources.remove(rem_pos);
                        println!("Remaining sources: {:?}", sources);
                    } else {
                        println!("Unable to borrow data sources");            
                    }    
                } else {
                    println!("Selection status is not single");
                }
            } else {
                println!("Unable to borrow selection status");
            }
            
            /*Self::remove_selected_mapping_page(
                plot_popover.clone(),
                mapping_menus.clone(),
                plot_view.clone()
            );*/
            
            /*mapping_popover.hide();
            if let Ok(mut pl) = plot_view.try_borrow_mut() {
                if let Ok(t_env) = table_env.try_borrow() {
                    if let Ok(menus) = mapping_menus.try_borrow() {
                        for m in menus.iter() {
                            if let Err(e) = m.update_data(&t_env, &mut pl) {
                                status_stack.update(Status::SqlErr(format!("{}", e)));
                                return;
                            }
                        }
                        status_stack.update(Status::Ok);
                    } else {
                        println!("Unable to retrieve mutable reference to mapping menus");
                    }
                } else {
                    println!("Unable to retrieve reference to table environment");
                }
            } else {
                println!("Unable retrieve mutable reference to plot view");
            }
            //plot_notebook.show_all();*/
        });
    }

    fn set_toggled_mappings(&self, mapping_types : &[&str]) {
        self.mapping_btns.iter()
            .for_each(|(_, m)| m.set_active(false) );
        self.mapping_btns.iter()
            .filter(|(name, _)| mapping_types.iter().find(|n| n == name).is_some() )
            .for_each(|(_, btn)| btn.set_active(true) );
    }

    /// Check which types are mapped to this set of column positions.
    /// Returns the types that are mapped to this set of columns (if any), the respective
    /// plot position, mapping (global) position, and mapping (local) position. Returns an empty vector otherwise.
    /// If a mapping type is not passed, the search is performed over all possible mapping types.
    fn check_mapped(
        selected : &[usize],
        tbl_ix : usize,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        sources : Rc<RefCell<Vec<DataSource>>>,
        mapping_type : Option<&str>
    ) -> Vec<SelectedMapping> {
        let mut mapped = Vec::new();
        if let Ok(sources) = sources.try_borrow() {
            for (i, source) in sources.iter().enumerate() {
                // if let Ok(source) = m.source.try_borrow() {
                if tbl_ix == source.tbl_pos.unwrap() {
                    let len_match = selected.len() == source.tbl_ixs.len();
                    let col_match = selected.iter()
                        .zip(source.tbl_ixs.iter())
                        .all(|(s, i)| *s == *i);
                    let type_match = match mapping_type {
                        Some(ty) => ty == &source.ty[..],
                        None => true
                    };
                    if len_match && col_match && type_match {
                        mapped.push(SelectedMapping{
                            ty : source.ty.clone(),
                            pl_ix : source.plot_ix,
                            global_ix : i,
                            local_ix : source.name.parse::<usize>().unwrap()
                        });
                    }
                }
                //} else {
                //    println!("Failed acquiring reference to data source");
                //}
            }
        } else {
            println!("Failed acquiring reference to mapping menus (check_mapped)");
        }
        mapped
    }

    fn set_active_mapping(
        unique_mapped : &SelectedMapping,
        selection : &Rc<RefCell<SelectionStatus>>,
        pl_view : &Rc<RefCell<PlotView>>,
        // plot_popover : &PlotPopover,
        group_toolbar : &GroupToolbar,
        add_mapping_btn : &ToolButton,
        edit_mapping_btn : &ToolButton,
        remove_mapping_btn : &ToolButton
    ) {
        *(selection.borrow_mut()) = SelectionStatus::Single(unique_mapped.clone());
        if let Ok(mut pl_view) = pl_view.try_borrow_mut() {
            pl_view.change_active_area(unique_mapped.pl_ix);
        } else {
            println!("Unable to acquire mutable reference to plot view");
            return;
        }
        /*plot_popover.set_active_mapping(
            unique_mapped.pl_ix,
            Some(unique_mapped.local_ix)
        );*/
        if !group_toolbar.get_sensitive() {
            group_toolbar.set_sensitive(true);
        }
        group_toolbar.switch_active_plot(unique_mapped.pl_ix);
        group_toolbar.set_sensitive(false);

        edit_mapping_btn.set_sensitive(true);
        remove_mapping_btn.set_sensitive(true);
        add_mapping_btn.set_sensitive(false);
    }

    fn build_add_mapping_toggles(
        builder : Builder,
        add_mapping_btn : ToolButton,
        edit_mapping_btn : ToolButton,
        remove_mapping_btn : ToolButton,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        plot_view : Rc<RefCell<PlotView>>,
        tbl_nb : TableNotebook,
        glade_def : Rc<HashMap<String, String>>,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        // plot_popover : PlotPopover,
        sources : Rc<RefCell<Vec<DataSource>>>,
        status_stack : StatusStack,
        sel_mapping : Rc<RefCell<String>>,
        selection : Rc<RefCell<SelectionStatus>>,
        group_toolbar : GroupToolbar
    ) -> (HashMap<String, ToggleToolButton>, Popover) {
        let add_mapping_popover : Popover = builder.get_object("add_mapping_popover").unwrap();
        let upper_mapping_toolbar : Toolbar = builder.get_object("upper_mapping_toolbar").unwrap();
        let lower_mapping_toolbar : Toolbar = builder.get_object("lower_mapping_toolbar").unwrap();
        let toolbars = [upper_mapping_toolbar, lower_mapping_toolbar];

        // Populating mappings HashMap
        let mapping_names = vec![
            String::from("bar"),
            String::from("line"),
            String::from("scatter"),
            String::from("text"),
            String::from("area"),
            String::from("surface")
        ];
        let mut mapping_btns = HashMap::new();
        let iter_names = mapping_names.iter().cloned();
        for (i, mapping) in iter_names.enumerate() {
            let img = Image::from_file(&(String::from("assets/icons/") +  &mapping + ".svg"));
            let btn : ToggleToolButton = ToggleToolButton::new();
            btn.set_icon_widget(Some(&img));
            mapping_btns.insert(mapping.to_string(), btn.clone());
            toolbars[i / 3].insert(&btn, (i % 3) as i32);
        }

        // Disable toggles alternatively / set add sensitive
        {
            let mapping_btns = mapping_btns.clone();
            let add_mapping_btn = add_mapping_btn.clone();
            let edit_mapping_btn = edit_mapping_btn.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            let tbl_nb = tbl_nb.clone();
            // let plot_popover = plot_popover.clone();
            // let mapping_menus = mapping_menus.clone();
            let plot_view = plot_view.clone();
            for (mapping_name, btn) in mapping_btns.iter().map(|(k, v)| (k.clone(), v.clone()) ) {
                let add_mapping_btn = add_mapping_btn.clone();
                let mapping_btns = mapping_btns.clone();
                let mapping_name = mapping_name.clone();
                let sel_mapping = sel_mapping.clone();
                // let mapping_menus = mapping_menus.clone();
                let edit_mapping_btn = edit_mapping_btn.clone();
                let remove_mapping_btn = remove_mapping_btn.clone();
                // let plot_popover = plot_popover.clone();
                let tbl_nb = tbl_nb.clone();
                let selection = selection.clone();
                let group_toolbar = group_toolbar.clone();
                let plot_view = plot_view.clone();
                let sources = sources.clone();
                btn.connect_toggled(move |btn| {
                    println!("{} toggled to {}", mapping_name, btn.get_active());
                    let (tbl_ix, selected_ix) = tbl_nb.selected_table_and_cols()
                        .unwrap_or((0, Vec::new()));
                    let toggled_btns : Vec<_> = mapping_btns.iter()
                        .filter(|(name, btn)| btn.get_active() )
                        .map(|(name, _)| name.to_string() )
                        .collect();
                    if toggled_btns.len() > 1 {
                        edit_mapping_btn.set_sensitive(false);
                        remove_mapping_btn.set_sensitive(false);
                        add_mapping_btn.set_sensitive(false);
                        return;
                    }
                    // Will always return zero or one, because we require that a single
                    // mapping type exists for any set of selected columns.

                    // Check if the current toggled type is already mapped
                    let this_mapped = Self::check_mapped(
                        &selected_ix[..],
                        tbl_ix,
                        // mapping_menus.clone(),
                        sources.clone(),
                        Some(&mapping_name)
                    );
                    let config_ans = Self::config_toggles_sensitive(
                        &add_mapping_btn,
                        &mapping_btns,
                        selected_ix.len()
                    );
                    if let Err(e) = config_ans {
                        println!("{}", e);
                    }
                    if btn.get_active() {
                        println!("This mapped: {:?}", this_mapped);
                        println!("Left toggled: {:?}", toggled_btns);
                        if let Ok(mut sel_mapping) = sel_mapping.try_borrow_mut() {
                            *sel_mapping = mapping_name.clone();
                        } else {
                            panic!("Failed to acquire mutable reference to selected mapping");
                        }
                        //println!("{} elements mapped to this column set ({}) selected", mapped.len(), selected.len());
                        match (this_mapped.len(), selected_ix.len()) {
                            (0, _n_cols) => {
                                *(selection.borrow_mut()) = SelectionStatus::New(0, mapping_name.clone());
                                add_mapping_btn.set_sensitive(true);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                                if !group_toolbar.get_sensitive() {
                                    group_toolbar.set_sensitive(true);
                                }
                                println!("Current active (toggled) : {:?}", group_toolbar.get_active_area());
                                // if !group_toolbar.any_selected() {
                                //    group_toolbar.set_active_default(None);
                                // }
                            },
                            (1, n_cols) => {
                                if n_cols >= 1 {
                                    let unique_mapped = &this_mapped[0];
                                    Self::set_active_mapping(
                                        &unique_mapped,
                                        &selection,
                                        &plot_view,
                                        // &plot_popover,
                                        &group_toolbar,
                                        &add_mapping_btn,
                                        &edit_mapping_btn,
                                        &remove_mapping_btn
                                    );
                                } else {
                                    *(selection.borrow_mut()) = SelectionStatus::None;
                                    edit_mapping_btn.set_sensitive(false);
                                    remove_mapping_btn.set_sensitive(false);
                                    add_mapping_btn.set_sensitive(false);
                                    group_toolbar.set_inactive();
                                    group_toolbar.set_sensitive(false);
                                }
                            },
                            (_n_mapped, _n_cols) => {
                                *(selection.borrow_mut()) = SelectionStatus::Multiple(this_mapped.clone());
                                add_mapping_btn.set_sensitive(false);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                                group_toolbar.set_inactive();
                                group_toolbar.set_sensitive(false);
                            }
                        }
                    } else {
                        println!("any_mapped");
                        let any_mapped = Self::check_mapped(
                            &selected_ix[..],
                            tbl_ix,
                            // mapping_menus.clone(),
                            sources.clone(),
                            None
                        );
                        println!("This mapped: {:?}", this_mapped);
                        println!("Any mapped: {:?}", any_mapped);
                        println!("Left toggled: {:?}", toggled_btns);
                        match toggled_btns.len() {
                            0 => {
                                let mut any_new_available = false;
                                for (btn_name, _) in mapping_btns.iter() {
                                    if !any_mapped.iter().find(|m| &m.ty[..] == &btn_name[..] ).is_some() {
                                        if !any_new_available {
                                            any_new_available = true;
                                        }
                                    }
                                }
                                if any_new_available {
                                    group_toolbar.set_sensitive(true);
                                } else {
                                    group_toolbar.set_sensitive(false);
                                }
                            },
                            1 => {
                                if let Ok(mut sel_mapping) = sel_mapping.try_borrow_mut() {
                                    let left_toggled = toggled_btns[0].clone();
                                    *sel_mapping = left_toggled;
                                } else {
                                    panic!("Failed to acquire mutable reference to selected mapping");
                                }
                                if any_mapped.len() >= 1 {
                                    let found_mapped = any_mapped.iter()
                                        .find(|sel| &sel.ty[..] == &toggled_btns[0][..] );
                                    if let Some(unique_mapped) = found_mapped {
                                        println!("Setting data of {:?} to plot popover", unique_mapped);
                                        Self::set_active_mapping(
                                            unique_mapped,
                                            &selection,
                                            &plot_view,
                                            // &plot_popover,
                                            &group_toolbar,
                                            &add_mapping_btn,
                                            &edit_mapping_btn,
                                            &remove_mapping_btn
                                        );
                                    } else {
                                        let add_on = this_mapped.len() == 0 ||
                                            &any_mapped[0].ty[..] != &toggled_btns[0];
                                        add_mapping_btn.set_sensitive(add_on);
                                        /*if add_on {
                                            group_toolbar.set_sensitive(true);
                                            group_toolbar.set_active_default(None);
                                        }*/
                                        edit_mapping_btn.set_sensitive(false);
                                        remove_mapping_btn.set_sensitive(false);
                                        /*if add_on {
                                            *(selection.borrow_mut()) = SelectionStatus::New;
                                        } else {
                                            *(selection.borrow_mut()) = SelectionStatus::None;
                                        }*/
                                    }
                                } else {
                                    add_mapping_btn.set_sensitive(false);
                                    edit_mapping_btn.set_sensitive(false);
                                    remove_mapping_btn.set_sensitive(false);
                                    // group_toolbar.set_inactive();
                                    // group_toolbar.set_sensitive(false);
                                    // *(selection.borrow_mut()) = SelectionStatus::None;
                                }
                            },
                            _ => {
                                add_mapping_btn.set_sensitive(false);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                            }
                        }
                    }
                });
            }
        }
        toolbars.iter().for_each(|t| t.show_all() );
        (mapping_btns, add_mapping_popover)
    }

    /*/// Remove all mappings if environment was updated with new data
    /// and mapping was not locked to received data.
    pub fn clear_invalid_mappings(
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let mut invalid : Vec<usize> = Vec::new();
        if let Ok(menus) = mapping_menus.try_borrow() {
            for (i, m) in menus.iter().enumerate() {
                if !m.source.borrow().valid {
                    invalid.push(i);
                    println!("Mapping {} is no longer valid", i);
                }
            }
        } else {
            println!("Unable to borrow from mapping menus");
        }
        for ix in invalid {
            let rem_ans = Self::remove_mapping_at_index(
                plot_popover.clone(),
                mapping_menus.clone(),
                plot_view.clone(),
                ix
            );
            if let Err(e) = rem_ans {
                println!("{}", e);
            }
        }
    }*/

    /*pub fn remove_mapping_at_index(
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_ix : usize
    ) -> Result<MappingMenu, String> {
        if let Ok(mut menus) = mapping_menus.try_borrow_mut() {
            if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                println!("Before removal: {:?}", menus);
                let (pl_ix, inner_mapping_ix) = plot_popover.remove_mapping_at_ix(mapping_ix);
                println!("Marked to remove: {} (plot {}, inner index {})", mapping_ix, pl_ix, inner_mapping_ix);
                // let pl_ix = menus[mapping_ix].plot_ix;
                let removed = menus.remove(mapping_ix);
                // assert!(menus[mapping_ix].plot_ix == pl_view.get_active_area());
                // let name = (mapping_ix).to_string();
                pl_view.update(&mut UpdateContent::RemoveMapping(
                    pl_ix,
                    inner_mapping_ix.to_string()
                ));
                println!("Remaining mappings: {:?}", menus);
                println!("Changing mappings names");
                let plot_mappings = menus
                    .iter_mut()
                    .filter(|m| m.plot_ix == pl_ix);
                for m in plot_mappings.skip(inner_mapping_ix) {
                    if let Some(Ok(old_ix)) = m.get_mapping_name().map(|n| n.parse::<usize>()) {
                        println!("Old index: {} New index: {}", old_ix, old_ix - 1);
                        m.set_mapping_name((old_ix - 1).to_string());
                    } else {
                        println!("Unable to parse mapping menu name to usize");
                    }
                }
                Ok(removed)
            } else {
                Err(format!("Could not get mutable reference to PlotView"))
            }
        } else {
            Err(format!("Unable to retrieve mutable reference to mapping_menus when removing page"))
        }
    }*/

    /*fn remove_selected_mapping_page(
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        if let Some(mapping_ix) = plot_popover.get_selected_mapping() {
            if let Err(e) = Self::remove_mapping_at_index(plot_popover, mapping_menus, plot_view, mapping_ix) {
                println!("Error removing mapping: {}", e);
            }
        } else {
            println!("No current mapping selected");
        }
    }*/

    /*pub fn set_edit_mapping_sensitive(&self, ncols : usize) -> Result<(), &'static str> {
        // TODO: Allow sensitive only when selected mapping applicable to current plot region.
        //let visible = self.sidebar_stack.get_visible_child_name()
        //    .ok_or("Unable to determine layout stack status" )?;
        //if &visible[..] == "layout" {
        /*if self.layout_loaded() {
            let page = self.notebook.get_property_page() as usize;
            if page <= 2 {
                self.layout_toolbar.edit_mapping_btn.set_sensitive(false);
                return Ok(());
            }*/
            /*let menus = self.mapping_menus.try_borrow()
                .map_err(|_| "Unable to retrieve reference to mapping menus")?;
            if let Some(m_type) = menus.get(page - 3).map(|m| m.mapping_type.clone() ) {
                match &m_type[..] {
                    "line" | "scatter" => {
                        if ncols == 2 {
                            self.layout_toolbar.edit_mapping_btn.set_sensitive(true);
                            return Ok(());
                        }
                    },
                    "bar" => {
                        if ncols == 1 {
                            self.layout_toolbar.edit_mapping_btn.set_sensitive(true);
                            return Ok(());
                        }
                    },
                    "text" | "surface" => {
                        if ncols == 3 {
                            self.layout_toolbar.edit_mapping_btn.set_sensitive(true);
                            return Ok(());
                        }
                    },
                    _ => return Err("Unrecognized mapping")
                }
            }
        }
        self.layout_toolbar.edit_mapping_btn.set_sensitive(false);*/
        if ncols == 0 {
            self.edit_mapping_btn.set_sensitive(false);
        }
        Ok(())
    }*/

    pub fn set_toggles_sensitive(&self, ncols : usize) -> Result<(), &'static str> {
        Self::config_toggles_sensitive(
            &self.add_mapping_btn,
            &self.mapping_btns,
            ncols
        )
    }

    pub fn update_mapping_status(
        &self,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        sources : Rc<RefCell<Vec<DataSource>>>,
        // plot_popover : &PlotPopover,
        selected : &[usize],
        tbl_ix : usize
    ) {
        let map_len = if let Ok(sources) = sources.try_borrow(){
            sources.len()
        } else {
            println!("Failed to acquire reference to mapping menus (to recover len)");
            return;
        };
        let mapped = Self::check_mapped(&selected[..], tbl_ix, sources.clone(), None);
        let types : Vec<_> = mapped.iter().map(|sel| sel.ty.as_str() ).collect();
        println!("{} elements mapped to this column set ({}) selected", mapped.len(), selected.len());

        if let Err(e) = self.set_toggles_sensitive(selected.len()) {
            println!("{}", e);
        }
        println!("Currently mapped data: {:?}", mapped);

        if let Some(last_mapped) = mapped.last() {

            // If multiple plots are present, we set the last one as the current one.
            // This does not have any effects for the user for mappend.len() > 1 since
            // the edit/remove buttons will only the sensitive for the case mapped.len() == 1
            self.set_toggled_mappings(&types[..]);

            // plot_popover.set_active_mapping(last_mapped.pl_ix, Some(last_mapped.local_ix));
            if mapped.len() == 1 {
                self.edit_mapping_btn.set_sensitive(true);
                self.remove_mapping_btn.set_sensitive(true);
            } else {
                self.edit_mapping_btn.set_sensitive(false);
                self.remove_mapping_btn.set_sensitive(false);
            }

            println!("Plot index: {} (update_mapping_status)", last_mapped.pl_ix);
            if !self.group_toolbar.get_sensitive() {
                self.group_toolbar.set_sensitive(true);
            }
            self.group_toolbar.switch_active_plot(last_mapped.pl_ix);
            self.group_toolbar.set_sensitive(false);

        } else {
            self.set_toggled_mappings(&[]);
            self.group_toolbar.set_active_default(None);
        }
    }

    /// The toggles that are sensitive is a function of the number of selected columns alone.
    /// When 2 columsn are selected only mappings that require two data columns are active,
    /// for example.
    fn config_toggles_sensitive(
        add_mapping_btn : &ToolButton,
        mapping_btns : &HashMap<String, ToggleToolButton>,
        ncols : usize
    ) -> Result<(), &'static str> {
        let sensitive : Vec<&str> = match ncols {
            1 => vec!["bar"],
            2 => vec!["line", "scatter"],
            3 => vec!["area", "text", "surface"],
            _ => vec![]
        };
        for (mapping, btn) in mapping_btns.iter() {
            if sensitive.iter().find(|n| *n == mapping).is_some() {
                if !btn.get_sensitive() {
                    btn.set_sensitive(true);
                }
            } else {
                if btn.get_sensitive() {
                    btn.set_sensitive(false);
                }
            }
        }
        Ok(())
    }

    /*pub fn update_selected_mapping(
        &self,
        tbl_nb : TableNotebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        mapping_pos : usize
    ) {
        if let Ok(mappings) = mapping_menus.try_borrow() {
            println!("Current mappings: {:?}", mappings);
            if let Some(mapping) = mappings.get(mapping_pos) {
                let source = mapping.source.borrow();
                let ixs = &source.ixs[..];
                let tbl_ixs = &source.tbl_ixs[..];
                let tbl_pos = if let Some(pos) = source.tbl_pos {
                    pos
                } else {
                    println!("Error recovering table position (value is None)");
                    return;
                };
                tbl_nb.unselect_all_tables();
                tbl_nb.set_page_index(tbl_pos);
                if let Some(tbl) = tbl_nb.expose_table(tbl_pos) {
                    tbl.set_selected(&tbl_ixs);
                    println!("Table indices: {:?}", tbl_ixs);
                    if let Some(ev) = tbl.expose_event_box(tbl_ixs[0]) {
                        self.mapping_popover.set_relative_to(Some(&ev));
                        self.mapping_popover.show();
                    } else {
                        println!("Could not retrieve event box for table {}", tbl_pos);
                    }
                } else {
                    println!("Could not expose table at position {}", tbl_pos);
                }
            } else {
                println!("No mapping at page {}", mapping_pos);
            }
        } else {
            println!("Could not borrow mapping_menus");
        }
    }*/


}

