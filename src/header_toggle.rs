use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
// use crate::tables::{source::EnvironmentSource, environment::TableEnvironment, environment::EnvironmentUpdate};
// use crate::{utils, table_notebook::TableNotebook };
use crate::status_stack::*;
// use crate::plots::plotview::plot_view::PlotView;
use crate::plots::plot_workspace::PlotWorkspace;
use crate::file_list::FileList;

#[derive(Debug, Clone)]
pub struct HeaderToggle {
    pub table_toggle : ToggleButton,
    pub plot_toggle : ToggleButton,
    pub query_toggle : ToggleButton
}

impl HeaderToggle {

    pub fn build(
        builder : &Builder,
        paned_pos : Rc<RefCell<i32>>,
        main_paned : Paned,
        // sidebar_stack : Stack,
        // content_stack : Stack,
        // status_stack : StatusStack,
        // workspace : PlotWorkspace,
        // file_list : FileList
    ) -> Self {
        let table_toggle : ToggleButton = builder.get_object("table_toggle").unwrap();
        let plot_toggle : ToggleButton = builder.get_object("plot_toggle").unwrap();
        let query_toggle : ToggleButton = builder.get_object("query_toggle").unwrap();

        {
            let main_paned_c = main_paned.clone();
            let paned_pos = paned_pos.clone();
            main_paned.connect_size_allocate(move |_paned_wid, _all| {
                if let Ok(mut s_pos) = paned_pos.try_borrow_mut() {
                    let new_pos = main_paned_c.get_position();
                    if new_pos > 0 {
                        *s_pos = new_pos
                    }
                } else {
                    println!("Error acquiring reference to main paned");
                }
            });
        }

        Self {
            query_toggle,
            table_toggle,
            plot_toggle
        }
    }

    pub fn connect_query_toggle(
        &self,
        status_stack : StatusStack,
        content_stack : Stack,
        file_list : FileList
    ) {
        let table_toggle = self.table_toggle.clone();
        let query_toggle = self.query_toggle.clone();
        let plot_toggle = self.plot_toggle.clone();
        query_toggle.connect_toggled(move |btn| {
            match btn.get_active() {
                false => {
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                    if plot_toggle.get_active() {
                        plot_toggle.toggled();
                    }
                    if table_toggle.get_active() {
                        table_toggle.toggled();
                    }
                },
                true => {
                    let page_name = if let Some(ix) = file_list.get_selected() {
                        format!("queries_{}", ix)
                    } else {
                        format!("no_queries")
                    };
                    println!("Setting visible: {}", page_name);
                    content_stack.set_visible_child_name(&page_name);
                    println!("Visible name: {:?}", content_stack.get_visible_child_name());
                    status_stack.show_alt();
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                    if table_toggle.get_active() {
                        table_toggle.set_active(false);
                    }
                    if plot_toggle.get_active() {
                        plot_toggle.set_active(false);
                    }
                }
            }
        });
    }

    pub fn connect_table_toggle(
        &self,
        content_stack : Stack,
        workspace : PlotWorkspace,
        status_stack : StatusStack
    ) {
        let query_toggle = self.query_toggle.clone();
        let plot_toggle = self.plot_toggle.clone();
        self.table_toggle.connect_toggled(move |btn| {
            match btn.get_active() {
                false => {
                    if !workspace.layout_loaded() {
                        status_stack.try_show_alt_or_connected();
                    }
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                    if plot_toggle.get_active() {
                        plot_toggle.toggled();
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                    }
                    if query_toggle.get_active() {
                        query_toggle.toggled();
                    }

                },
                true => {
                    content_stack.set_visible_child_name("tables");
                    status_stack.try_show_alt_or_connected();
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                    if plot_toggle.get_active() {
                        plot_toggle.set_active(false);
                    }
                    if query_toggle.get_active() {
                        query_toggle.set_active(false);
                    }
                }
            }
        });
    }

    pub fn connect_plot_toggle(
        &self,
        content_stack : Stack,
        workspace : PlotWorkspace,
        status_stack : StatusStack
    ) {
        let table_toggle = self.table_toggle.clone();
        let query_toggle = self.query_toggle.clone();
        self.plot_toggle.connect_toggled(move |btn| {
            match btn.get_active() {
                false => {
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                    if table_toggle.get_active() {
                        table_toggle.toggled();
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                    }
                    if query_toggle.get_active() {
                        query_toggle.toggled();
                    }
                },
                true => {
                    // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                    content_stack.set_visible_child_name("plot");
                    if let Ok(pl_view) = workspace.pl_view.try_borrow() {
                        pl_view.redraw();
                    } else {
                        println!("Failed to acquire lock over plot")
                    }
                    if table_toggle.get_active() {
                        table_toggle.set_active(false);
                    }
                    if query_toggle.get_active() {
                        query_toggle.set_active(false);
                    }
                    status_stack.try_show_alt();
                }
            }
        });
    }

}
