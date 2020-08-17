use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::{environment::TableEnvironment};
use crate::plots::plotview::GroupSplit;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
use std::fs::File;
use std::io::Read;
use super::design_menu::*;
use super::scale_menu::*;
use super::layout_window::*;
use super::mapping_menu::*;
use super::plot_popover::*;
use std::collections::HashMap;
use crate::utils;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
use std::default::Default;
use crate::plots::plot_workspace::PlotWorkspace;

/// LayoutToolbar encapsulates the logic for the popover when the user right-click
/// some table columns to add, edit or remove mappings.
#[derive(Debug, Clone)]
pub struct LayoutToolbar {
    pub add_mapping_btn : ToolButton,
    pub edit_mapping_btn : ToolButton,
    pub clear_layout_btn : ToolButton,
    pub remove_mapping_btn : ToolButton,
    pub mapping_popover : Popover,
    pub mapping_btns : HashMap<String, ToggleToolButton>
}

impl LayoutToolbar {

    pub fn build(
        builder : Builder,
        status_stack : StatusStack,
        sidebar_stack : Stack,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        layout_path : Rc<RefCell<Option<String>>>,
        plot_toggle : ToggleButton,
        glade_def : Rc<HashMap<String, String>>,
        sel_mapping : Rc<RefCell<String>>
    ) -> Self {
        let layout_toolbar : Toolbar = builder.get_object("layout_toolbar").unwrap();
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
            let mapping_menus = mapping_menus.clone();
            //let notebook = plot_notebook.clone();
            let status_stack = status_stack.clone();
            let layout_path = layout_path.clone();
            clear_layout_btn.connect_clicked(move |btn| {
                //TODO toggle group toolbar to single
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout-unique.xml")));
                    sidebar_stack.set_visible_child_name("empty");
                    *(layout_path.borrow_mut()) = None;
                    status_stack.show_curr_status();
                    if let Ok(mut mappings) = mapping_menus.try_borrow_mut() {
                        mappings.clear();
                    } else {
                        println!("Error retrieving mapping menus");
                    }

                    // Use stack logic instead
                    /*let children = notebook.get_children();
                    for i in 3..children.len() {
                        if let Some(c) = children.get(i) {
                            notebook.remove(c);
                        } else {
                            println!("Unable to clear notebook");
                        }
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
            mapping_menus.clone(),
            plot_popover.clone(),
            status_stack.clone(),
            sel_mapping.clone()
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
            mapping_popover
        };

        {
            let tbl_nb = tbl_nb.clone();
            let layout_toolbar = layout_toolbar.clone();
            let plot_popover = plot_popover.clone();
            let mapping_menus = mapping_menus.clone();
            layout_toolbar.mapping_popover.clone().connect_show(move |wid| {

                // This is necessary because the table set_sensitive=false propagates
                // to the popover when new data arrives
                if !wid.get_sensitive() {
                    wid.set_sensitive(true);
                }

                let selected = tbl_nb.full_selected_cols();
                layout_toolbar.set_add_or_edit_mapping_sensitive(
                    mapping_menus.clone(),
                    &plot_popover,
                    &selected[..]
                );
            });
        }

        layout_toolbar
    }

    pub fn connect_add_mapping_clicked(
        &self,
        plot_popover : PlotPopover,
        glade_def : Rc<HashMap<String, String>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        status_stack : StatusStack,
        sel_mapping : Rc<RefCell<String>>
    ) {
        let edit_mapping_btn = self.edit_mapping_btn.clone();
        let remove_mapping_btn = self.remove_mapping_btn.clone();
        let mapping_btns = self.mapping_btns.clone();
        self.add_mapping_btn.connect_clicked(move|btn| {
            let m = sel_mapping.borrow();
            PlotWorkspace::add_mapping_from_type(
                glade_def.clone(),
                &m[..],
                table_env.clone(),
                tbl_nb.clone(),
                plot_view.clone(),
                mapping_menus.clone(),
                plot_popover.clone(),
                status_stack.clone()
            );
            btn.set_sensitive(false);
            edit_mapping_btn.set_sensitive(true);
            remove_mapping_btn.set_sensitive(true);
            plot_popover.update_nav_sensitive();
            //plot_popover.update_stack();
        });
    }

    pub fn connect_edit_mapping_clicked(
        &self,
        plot_toggle : ToggleButton,
        plot_popover : PlotPopover,
        pl_view : Rc<RefCell<PlotView>>,
        tbl_nb : TableNotebook
    ) {
        let mapping_popover = self.mapping_popover.clone();
        self.edit_mapping_btn.connect_clicked(move |btn| {
            plot_toggle.set_active(true);
            mapping_popover.hide();
            let (x, y, w, h) = PlotWorkspace::get_active_coords(&pl_view.borrow());
            tbl_nb.unselect_all_tables();
            plot_popover.show_at(x, y, w, h);
            // Disable most recent toggle
            // mapping_btns.iter()
            //    .filter(|(name, btn)| &name[..] == &m[..] )
            //    .for_each(|(name, btn)| btn.set_active(false) );
        });
    }

    pub fn connect_remove_mapping_clicked(
        &self,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>,
        plot_popover : PlotPopover,
        status_stack : StatusStack,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        mapping_popover : Popover,
    ) {
        self.mapping_popover.hide();
        self.remove_mapping_btn.connect_clicked(move |_| {
            Self::remove_selected_mapping_page(
                plot_popover.clone(),
                mapping_menus.clone(),
                plot_view.clone()
            );
            tbl_nb.unselect_all_tables();
            mapping_popover.hide();
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
            //plot_notebook.show_all();
        });
    }

    fn set_toggled_mappings(&self, mapping_types : &[&str]) {
        self.mapping_btns.iter()
            .for_each(|(_, m)| m.set_active(false) );
        self.mapping_btns.iter()
            .filter(|(name, _)| mapping_types.iter().find(|n| n == name).is_some() )
            .for_each(|(_, btn)| btn.set_active(true) );
    }

    /// Check which types are mapped to this set of column positions (linear index).
    /// Returns the types that are mapped to this set of columns (if any), the respective
    /// plot position, and mapping (linear) position. Returns an empty vector otherwise.
    /// If a mapping type is not passed, the search is performed over all possible mapping types.
    fn check_mapped(
        selected : &[usize],
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        mapping_type : Option<&str>
    ) -> Vec<(String, usize, usize)> {
        let mut mapped = Vec::new();
        if let Ok(menus) = mapping_menus.try_borrow() {
            for (i, m) in menus.iter().enumerate() {
                if let Ok(source) = m.source.try_borrow() {
                    let len_match = selected.len() == source.ixs.len();
                    let col_match = selected.iter()
                        .zip(source.ixs.iter())
                        .all(|(s, i)| *s == *i);
                    let type_match = match mapping_type {
                        Some(ty) => ty == &m.mapping_type[..],
                        None => true
                    };
                    if len_match && col_match && type_match {
                        mapped.push((m.mapping_type.clone(), m.plot_ix, i));
                    }
                } else {
                    println!("Failed acquiring reference to data source");
                }
            }
        } else {
            println!("Failed acquiring reference to mapping menus (check_mapped)");
        }
        mapped
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
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : PlotPopover,
        status_stack : StatusStack,
        sel_mapping : Rc<RefCell<String>>
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
            let plot_popover = plot_popover.clone();
            let mapping_menus = mapping_menus.clone();
            for (mapping_name, btn) in mapping_btns.iter().map(|(k, v)| (k.clone(), v.clone()) ) {
                let add_mapping_btn = add_mapping_btn.clone();
                let mapping_btns = mapping_btns.clone();
                let mapping_name = mapping_name.clone();
                let sel_mapping = sel_mapping.clone();
                let mapping_menus = mapping_menus.clone();
                let edit_mapping_btn = edit_mapping_btn.clone();
                let remove_mapping_btn = remove_mapping_btn.clone();
                let plot_popover = plot_popover.clone();
                let tbl_nb = tbl_nb.clone();
                btn.connect_toggled(move |btn| {
                    println!("{} toggled to {}", mapping_name, btn.get_active());
                    let selected = tbl_nb.full_selected_cols();
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

                    let this_mapped = Self::check_mapped(
                        &selected[..],
                        mapping_menus.clone(),
                        Some(&mapping_name)
                    );
                    Self::config_toggles_sensitive(
                        &add_mapping_btn,
                        &mapping_btns,
                        selected.len()
                    );
                    if btn.get_active() {
                        println!("This mapped: {:?}", this_mapped);
                        println!("Left toggled: {:?}", toggled_btns);
                        if let Ok(mut sel_mapping) = sel_mapping.try_borrow_mut() {
                            *sel_mapping = mapping_name.clone();
                        } else {
                            println!("Failed to acquire mutable reference to selected mapping");
                        }
                        //println!("{} elements mapped to this column set ({}) selected", mapped.len(), selected.len());
                        match (this_mapped.len(), selected.len()) {
                            (0, n) => {
                                add_mapping_btn.set_sensitive(true);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                            },
                            (1, n) => {
                                if n >= 1 {
                                    plot_popover.set_active_mapping(this_mapped[0].1, Some(this_mapped[0].2));
                                    edit_mapping_btn.set_sensitive(true);
                                    remove_mapping_btn.set_sensitive(true);
                                    add_mapping_btn.set_sensitive(false);
                                } else {
                                    edit_mapping_btn.set_sensitive(false);
                                    remove_mapping_btn.set_sensitive(false);
                                    add_mapping_btn.set_sensitive(false);
                                }
                            },
                            _ => {
                                add_mapping_btn.set_sensitive(false);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                            }
                        }
                    } else {
                        println!("any_mapped");
                        let any_mapped = Self::check_mapped(
                            &selected[..],
                            mapping_menus.clone(),
                            None
                        );
                        println!("This mapped: {:?}", this_mapped);
                        println!("Any mapped: {:?}", any_mapped);
                        println!("Left toggled: {:?}", toggled_btns);
                        if toggled_btns.len() == 1 && any_mapped.len() >= 1 {
                            let found_mapped = any_mapped.iter()
                                .find(|(name, _, _)| &name[..] == &toggled_btns[0][..] );
                            if let Some(mapped) = found_mapped {
                                println!("Setting data of {:?} to plot popover", mapped);
                                plot_popover.set_active_mapping(mapped.1, Some(mapped.2));
                                add_mapping_btn.set_sensitive(false);
                                edit_mapping_btn.set_sensitive(true);
                                remove_mapping_btn.set_sensitive(true);
                            } else {
                                let add_on = toggled_btns.len() == 1 &&
                                    (this_mapped.len() == 0 || &any_mapped[0].0 != &toggled_btns[0]);
                                add_mapping_btn.set_sensitive(add_on);
                                edit_mapping_btn.set_sensitive(false);
                                remove_mapping_btn.set_sensitive(false);
                            }
                        } else {
                            add_mapping_btn.set_sensitive(false);
                            edit_mapping_btn.set_sensitive(false);
                            remove_mapping_btn.set_sensitive(false);
                        }
                    }
                });
            }
        }
        toolbars.iter().for_each(|t| t.show_all() );
        (mapping_btns, add_mapping_popover)
    }

    /// Remove all mappings if environment was updated with new data
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
            Self::remove_mapping_at_index(
                plot_popover.clone(),
                mapping_menus.clone(),
                plot_view.clone(),
                ix
            );
        }
    }

    pub fn remove_mapping_at_index(
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_ix : usize
    ) {
        if let Ok(mut menus) = mapping_menus.try_borrow_mut() {
            if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                plot_popover.remove_mapping_at_ix(mapping_ix);
                println!("Marked to remove: {}", mapping_ix);
                menus.remove(mapping_ix);
                let name = (mapping_ix).to_string();
                pl_view.update(&mut UpdateContent::RemoveMapping(name));
                for m in menus.iter_mut().skip(mapping_ix) {
                    if let Some(Ok(old_ix)) = m.get_mapping_name().map(|n| n.parse::<usize>()) {
                        println!("Old index: {} New index: {}", old_ix, old_ix - 1 );
                        m.set_mapping_name((old_ix - 1).to_string());
                    } else {
                        println!("Unable to parse mapping menu name to usize");
                    }
                }
            } else {
                println!("Could not get mutable reference to PlotView")
            }
        } else {
            println!("Unable to retrieve mutable reference to mapping_menus when removing page");
        }
    }

    fn remove_selected_mapping_page(
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let mapping_ix = plot_popover.get_selected_mapping();
        Self::remove_mapping_at_index(plot_popover, mapping_menus, plot_view, mapping_ix);
    }

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

    pub fn set_add_or_edit_mapping_sensitive(
        &self,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_popover : &PlotPopover,
        selected : &[usize],
    ) {
        let map_len = if let Ok(mappings) = mapping_menus.try_borrow(){
            mappings.len()
        } else {
            println!("Failed to acquire reference to mapping menus (to recover len)");
            return;
        };
        println!("add_or_edit_sensitive");
        let mapped = Self::check_mapped(&selected[..], mapping_menus.clone(), None);
        let types : Vec<_> = mapped.iter().map(|(t, _, _)| t.as_str() ).collect();
        println!("{} elements mapped to this column set ({}) selected", mapped.len(), selected.len());

        self.set_toggles_sensitive(selected.len());
        println!("Currently mapped data: {:?}", mapped);

        if let Some((_, plot_ix, mapping_ix)) = mapped.last() {

            // If multiple plots are present, we set the last one as the current one.
            // This does not have any effects for the user for mappend.len() > 1 since
            // the edit/remove buttons will only the sensitive for the case mapped.len() == 1
            self.set_toggled_mappings(&types[..]);

            plot_popover.set_active_mapping(*plot_ix, Some(*mapping_ix));
            if mapped.len() == 1 {
                self.edit_mapping_btn.set_sensitive(true);
                self.remove_mapping_btn.set_sensitive(true);
            } else {
                self.edit_mapping_btn.set_sensitive(false);
                self.remove_mapping_btn.set_sensitive(false);
            }
        } else {
            self.set_toggled_mappings(&[]);
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

    pub fn update_selected_mapping(
        &self,
        tbl_nb : TableNotebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        mapping_pos : usize
    ) {
        if let Ok(mappings) = mapping_menus.try_borrow() {
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
    }


}

