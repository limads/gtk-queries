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
use super::layout_toolbar::*;
use super::mapping_menu::*;
use super::layout_window::*;
use std::collections::HashMap;
use crate::utils;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
use std::default::Default;
use gdk::EventButton;

#[derive(Clone, Debug)]
pub struct MappingSelection {

    /// What is the currently selected plot, re-calculated every time the user
    /// clicks the mapping menu at a different region.
    pub plot_ix : usize,

    /// Carries what global mapping indices are valid for each plot (0-3).
    /// Those indices are the same stack indices used to switch the mapping stack.
    pub valid_ix : Vec<Vec<usize>>,

    /// This indexes the inner vectors of valid_ix, and is
    /// controled by the left/right arrows at PlotPopover.
    pub curr_ix : usize
}

/// PlotPopover encapsulates the logic for when the user right-clicks the scales or content
/// of one of the plots. It interoperates heavily with LayoutToolbar during the selection
/// and edition of mappings.
#[derive(Clone, Debug)]
pub struct PlotPopover {
    pub data_popover : Popover,
    pub scale_x_popover : Popover,
    pub scale_y_popover : Popover,
    pub mapping_stack : Stack,
    forward_btn : ToolButton,
    backward_btn : ToolButton,
    pub tbl_btn : ToolButton,
    sel_mapping : Rc<RefCell<MappingSelection>>
}

impl PlotPopover {

    pub fn new(builder : &Builder) -> Self {
        let data_popover : Popover = builder.get_object("mapping_select_popover").unwrap();
        let scale_x_popover : Popover = builder.get_object("scale_x_popover").unwrap();
        let scale_y_popover : Popover = builder.get_object("scale_y_popover").unwrap();

        let mapping_stack : Stack = builder.get_object("mapping_stack").unwrap();
        let mapping_select_toolbar : Toolbar = builder.get_object("mapping_select_toolbar").unwrap();

        let img_forward = Image::from_icon_name(Some("go-next"), IconSize::SmallToolbar);
        let img_backward = Image::from_icon_name(Some("go-previous"), IconSize::SmallToolbar);
        let tab_img_path = String::from("assets/icons/grid-black.svg");
        let img_table = Image::from_file(&tab_img_path[..]);

        let forward_btn : ToolButton = ToolButton::new(Some(&img_forward), None);
        let backward_btn : ToolButton = ToolButton::new(Some(&img_backward), None);
        let tbl_btn : ToolButton = ToolButton::new(Some(&img_table), None);
        forward_btn.set_sensitive(false);
        backward_btn.set_sensitive(false);
        tbl_btn.set_sensitive(false);
        let sel = MappingSelection {
            plot_ix : 0,
            valid_ix : vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            curr_ix : 0
        };
        let sel_mapping = Rc::new(RefCell::new(sel));
        mapping_select_toolbar.insert(&tbl_btn, 0);
        mapping_select_toolbar.insert(&backward_btn, 1);
        mapping_select_toolbar.insert(&forward_btn, 2);
        mapping_select_toolbar.show_all();
        let plot_popover = Self {
            tbl_btn,
            backward_btn,
            forward_btn,
            mapping_stack,
            sel_mapping,
            data_popover,
            scale_x_popover,
            scale_y_popover
        };

        {
            let plot_popover = plot_popover.clone();
            plot_popover.data_popover.clone().connect_show(move |wid| {
                plot_popover.update_nav_sensitive();
                plot_popover.update_stack();
            });
        }

        {
            let plot_popover = plot_popover.clone();
            plot_popover.data_popover.clone().connect_closed(move |pop| {
                /*if let Ok(mut sel_mapping) = plot_popover.sel_mapping.try_borrow_mut() {
                    sel_mapping.curr_ix = 0;
                    println!("Current selection (closed): {:?}", sel_mapping);
                } else {
                    println!("Failed acquiring mutable reference to sel_mapping");
                }*/
                // plot_popover.update_nav_sensitive();
                // plot_popover.update_stack();
            });
        }

        {
            let sel_mapping = plot_popover.sel_mapping.clone();
            let mapping_stack = plot_popover.mapping_stack.clone();
            let plot_popover = plot_popover.clone();
            plot_popover.forward_btn.clone().connect_clicked(move |btn| {
                Self::navigate(sel_mapping.clone(), mapping_stack.clone(), btn.clone(), true);
                plot_popover.update_nav_sensitive();
            });
        }

        {
            let sel_mapping = plot_popover.sel_mapping.clone();
            let mapping_stack = plot_popover.mapping_stack.clone();
            let plot_popover = plot_popover.clone();
            plot_popover.backward_btn.clone().connect_clicked(move |btn| {
                Self::navigate(sel_mapping.clone(), mapping_stack.clone(), btn.clone(), false);
                plot_popover.update_nav_sensitive();
            });
        }

        plot_popover
    }

    pub fn clear(&self) {
        self.mapping_stack.set_visible_child_name("empty");
        self.mapping_stack.show_all();
        let stack_children = self.mapping_stack.get_children();
        for child in stack_children.iter().skip(1) {
            self.mapping_stack.remove(child);
        }
        let mut sel = self.sel_mapping.borrow_mut();
        sel.plot_ix = 0;
        for ix_vec in sel.valid_ix.iter_mut() {
            ix_vec.clear();
        }
        sel.curr_ix = 0;
        self.forward_btn.set_sensitive(false);
        self.backward_btn.set_sensitive(false);
        self.tbl_btn.set_sensitive(false);
    }

    fn navigate(
        sel_mapping : Rc<RefCell<MappingSelection>>,
        mapping_stack : Stack,
        btn : ToolButton,
        forward: bool
    ) {
        if let Ok(mut sel) = sel_mapping.try_borrow_mut() {
            if sel.curr_ix < sel.valid_ix[sel.plot_ix].len()  {
                if forward {
                    sel.curr_ix += 1;
                } else {
                    sel.curr_ix -= 1;
                }
                let children = mapping_stack.get_children();
                if let Some(child) = children.get(sel.valid_ix[sel.plot_ix][sel.curr_ix]) {
                    mapping_stack.set_visible_child(child);
                } else {
                    println!("Child not found at index {:?}", sel);
                }
                println!("Current selection (navigate): {:?}", sel);
            } else {
                println!("Extrapolated plot index (Curr ix : {:?})", sel);
            }
        } else {
            println!("Failed to acquire mutable borrow over selected mapping/selected mapping empty");
        }
    }

    pub fn set_active_mapping(&self, plot_ix : usize, curr_ix : Option<usize>) {
        if let Ok(mut sel) = self.sel_mapping.try_borrow_mut() {
            sel.plot_ix = plot_ix;
            sel.curr_ix = curr_ix.unwrap_or(0);
            let children = self.mapping_stack.get_children();
            if children.len() == 1 {
                self.mapping_stack.set_visible_child(children.get(0).unwrap());
            } else {
                if let Some(child) = children.get(sel.valid_ix[plot_ix][sel.curr_ix]) {
                    self.mapping_stack.set_visible_child(child);
                } else {
                    println!("No child found at index {}", sel.valid_ix[plot_ix][sel.curr_ix]);
                }
            }
            println!("Current selection (set_active): {:?}", sel);
        } else {
            println!("Failed to aquire mutable reference to sel_mapping");
        }
    }

    pub fn update_nav_sensitive(&self) {
        if let Ok(sel_mapping) = self.sel_mapping.try_borrow() {
            let curr_sz = sel_mapping.valid_ix[sel_mapping.plot_ix].len();
            match curr_sz {
                0 => {
                    self.tbl_btn.set_sensitive(false);
                    self.forward_btn.set_sensitive(false);
                    self.backward_btn.set_sensitive(false);
                },
                1 => {
                    self.tbl_btn.set_sensitive(true);
                    self.forward_btn.set_sensitive(false);
                    self.backward_btn.set_sensitive(false);
                },
                n => {
                    if sel_mapping.curr_ix == 0 {
                        self.tbl_btn.set_sensitive(true);
                        self.backward_btn.set_sensitive(false);
                        self.forward_btn.set_sensitive(true);
                    } else {
                        if sel_mapping.curr_ix == curr_sz - 1 {
                            self.tbl_btn.set_sensitive(true);
                            self.backward_btn.set_sensitive(true);
                            self.forward_btn.set_sensitive(false);
                        } else {
                            self.tbl_btn.set_sensitive(true);
                            self.backward_btn.set_sensitive(true);
                            self.forward_btn.set_sensitive(true);
                        }
                    }
                }
            }
            println!("Current size (update_nav_sensitive): {}", curr_sz);
        } else {
            println!("Unable to acquire reference to selected mapping");
        }
    }

    pub fn update_stack(&self) {
        if let Ok(sel) = self.sel_mapping.try_borrow() {
            let children = self.mapping_stack.get_children();
            let active_child = if children.len() == 1 {
                children.get(0).unwrap()
            } else {
                let valid_ixs = &sel.valid_ix[sel.plot_ix];
                println!("Valid indices: {:?}", valid_ixs);
                if valid_ixs.len() == 0 {
                    children.get(0).unwrap()
                } else {
                    if let Some(child) = children.get(valid_ixs[sel.curr_ix]) {
                        child
                    } else {
                        println!("No child at index {} to update stack", valid_ixs[sel.curr_ix]);
                        return;
                    }
                }
            };
            println!("Current selection (update_stack): {:?}", sel);
            self.mapping_stack.set_visible_child(active_child);
        } else {
            println!("Unable to borrow selected mapping");
        }
    }

    pub fn add_mapping(&self, m : &MappingMenu) {
        let mapping_stack = &self.mapping_stack;
        mapping_stack.add(&m.get_parent());
        let children = mapping_stack.get_children();
        let n_mappings = children.len() - 1;
        if let Ok(mut sel_mapping) = self.sel_mapping.try_borrow_mut() {
            let pl_ix = m.plot_ix;
            sel_mapping.valid_ix[pl_ix].push(n_mappings);
            sel_mapping.curr_ix = n_mappings - 1;
            sel_mapping.plot_ix = pl_ix;
            println!("n_mappings: {}", n_mappings);
            println!("n_children: {}", n_mappings);
            mapping_stack.set_visible_child(children.get(children.len() - 1).unwrap());
            println!("Current selection (add_mapping): {:?}", sel_mapping);
        } else {
            println!("Failed acquiring mutable reference to selected mapping");
        }
        self.update_stack();
        self.update_nav_sensitive();
    }

    /// Removes the selected mapping
    pub fn remove_mapping_at_ix(&self, ix : usize) {
        if let Ok(mut sel_mapping) = self.sel_mapping.try_borrow_mut() {
            // let offset_mapping_ix = sel_mapping.valid_ix[pl_ix][curr_ix];

            let children = self.mapping_stack.get_children();
            if let Some(c) = children.get(ix + 1) {
                self.mapping_stack.remove(c);
                self.data_popover.hide();
            } else {
                panic!("Invalid child position: {}", ix);
            }
            let pl_ix = sel_mapping.valid_ix
                .iter()
                .position(|v| v.iter().position(|i| *i == ix+1 ).is_some() )
                //.position(|v| v.iter().position(|i| *i == ix ).is_some() )
                .unwrap();
            let m_ix = sel_mapping.valid_ix[pl_ix]
                .iter()
                .position(|i| *i == ix+1 )
                //.position(|i| *i == ix )
                .unwrap();
            println!("Removing at plot {} at plot index {}", pl_ix, m_ix);
            sel_mapping.valid_ix[pl_ix].remove(m_ix);
            for m in sel_mapping.valid_ix[pl_ix].iter_mut().skip(m_ix) {
                *m -= 1;
            }
            sel_mapping.curr_ix = 0;
            if sel_mapping.valid_ix[pl_ix].len() == 0 {
                sel_mapping.plot_ix = 0;
            }
            println!("Current selection (remove_selected_mapping): {:?}", sel_mapping);
        } else {
            panic!("Unable to retrieve mutable reference to selected mapping");
        }
        self.update_stack();
        self.update_nav_sensitive();
    }

    /// Show the data popover exclusively at an arbitrary position.
    pub fn show_at(&self, x : i32, y : i32, w : i32, h : i32) {
        self.data_popover.set_pointing_to(&Rectangle{ x, y, width : 10, height : 10});
        println!("Pointing popover to: {:?}", (x, y, w, h));
        self.scale_x_popover.hide();
        self.scale_y_popover.hide();
        self.data_popover.show();
    }

    fn within_rect(x : f64, y : f64, w : i32, h : i32, rect : (f64, f64, f64, f64)) -> bool {
        x > w as f64 * rect.0 && x < w as f64 * rect.1 &&
            y > h as f64 *rect.2 && y < h as f64 * rect.3
    }

    /// Show either the x/y scale popovers or the data popover from a click,
    /// dependin on where the click was made.
    pub fn show_from_click(&self, ev : &EventButton, w : i32, h : i32, layout : GroupSplit, active_area : usize) {
        let (x, y) = ev.get_position();
        //let w = ev.get_allocation().width; //wid (draw area)
        //let h = ev.get_allocation().height; //wid (draw area)

        let (x_rect, y_rect) = match (layout, active_area) {
            (GroupSplit::Unique, _) => ((0.0, 1.0, 0.9, 1.0), (0.0, 0.1, 0.0, 1.0)),
            (GroupSplit::Vertical, 0) => ((0.0, 0.5, 0.9, 1.0), (0.0, 0.1, 0.0, 1.0)),
            (GroupSplit::Vertical, 1) => ((0.5, 1.0, 0.9, 1.0), (0.5, 0.6, 0.0, 1.0)),
            (GroupSplit::Horizontal, 0) => ((0.0, 1.0, 0.4, 0.5), (0.0, 0.1, 0.0, 0.5)),
            (GroupSplit::Horizontal, 1) => ((0.0, 1.0, 0.9, 1.0), (0.0, 0.1, 0.5, 1.0)),
            (GroupSplit::Four, 0) => ((0.0, 0.5, 0.4, 0.5), (0.0, 0.1, 0.0, 0.5)),
            (GroupSplit::Four, 1) => ((0.5, 1.0, 0.4, 0.5), (0.5, 0.6, 0.5, 1.0)),
            (GroupSplit::Four, 2) => ((0.0, 0.5, 0.9, 1.0), (0.0, 0.1, 0.5, 1.0)),
            (GroupSplit::Four, 3) => ((0.5, 1.0, 0.9, 1.0), (0.5, 0.6, 0.5, 1.0)),
            _ => unimplemented!()
        };
        if ev.get_button() == 3 {
            if Self::within_rect(x, y, w, h, y_rect) {
                self.scale_y_popover.set_pointing_to(&Rectangle{
                    x : x as i32,
                    y : y as i32,
                    width : 10,
                    height : 10
                });
                self.scale_y_popover.show();
                self.scale_x_popover.hide();
                self.data_popover.hide();
            } else {
                if Self::within_rect(x, y, w, h, x_rect) {
                    self.scale_x_popover.set_pointing_to(&Rectangle{
                        x : x as i32,
                        y : y as i32,
                        width : 10,
                        height : 10
                    });
                    self.scale_x_popover.show();
                    self.scale_y_popover.hide();
                    self.data_popover.hide();
                } else {
                    self.data_popover.set_pointing_to(&Rectangle{
                        x : (w as f64 * 0.5) as i32,
                        y : (h as f64 * 0.5) as i32,
                        width : 10,
                        height : 10
                    });
                    self.data_popover.show();
                    self.scale_x_popover.hide();
                    self.scale_y_popover.hide();
                }
            }
        } else {
            self.scale_x_popover.hide();
            self.scale_y_popover.hide();
            self.data_popover.hide();
        }
    }

    /// Gets the currently selected mapping, with respect to the global mapping
    /// vector. Since all mapping indices are offset by +1 (due to a first stack
    /// element being the empty plot label), subtract one from the valid index
    /// to get the global mapping index.
    pub fn get_selected_mapping(&self) -> usize {
        if let Ok(sel) = self.sel_mapping.try_borrow() {
            sel.valid_ix[sel.plot_ix][sel.curr_ix] - 1
        } else {
            panic!("Failed acquiring reference to selected mapping");
        }
    }

}


