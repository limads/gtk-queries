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
use super::plot_workspace::PlotWorkspace;
use std::io::Write;

#[derive(Clone)]
pub struct LayoutWindow {
    pub open_btn : Button,
    pub save_btn : Button,
    pub query_btn : Button,
    pub clear_btn : Button,
    pub xml_save_dialog : FileChooserDialog,
    pub xml_load_dialog : FileChooserDialog,
    toggles : HashMap<GroupSplit, ToggleToolButton>,
    file_combo : ComboBoxText,
}

const ALL_LAYOUTS : [GroupSplit; 8] = [
    GroupSplit::Unique,
    GroupSplit::Horizontal,
    GroupSplit::Vertical,
    GroupSplit::Four,
    GroupSplit::ThreeLeft,
    GroupSplit::ThreeTop,
    GroupSplit::ThreeRight,
    GroupSplit::ThreeBottom
];

const ALL_PATHS : [&'static str; 8] = [
    "layout-unique",
    "layout-horiz",
    "layout-vert",
    "layout-four",
    "layout-three-left",
    "layout-three-top",
    "layout-three-right",
    "layout-three-bottom"
];

impl LayoutWindow {

    pub fn get_recent_paths() {

    }

    pub fn push_recent_path() {

    }

    pub fn set_sensitive_at_index(menus : &[MappingMenu], ix : usize) {
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

    fn build_save_dialog(
        builder : &Builder,
        save_btn : Button,
        layout_file_combo : ComboBoxText,
        pl_view : Rc<RefCell<PlotView>>
    ) -> FileChooserDialog {
        let xml_save_dialog : FileChooserDialog = builder.get_object("xml_save_dialog").unwrap();
        xml_save_dialog.connect_response(move |dialog, resp| {
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        if let Some(ext) = path.as_path().extension().map(|ext| ext.to_str().unwrap_or("")) {
                            match ext {
                                "xml" => {
                                    if let Ok(pl) = pl_view.try_borrow() {
                                        if let Ok(mut f) = File::create(path) {
                                            let content = pl.plot_group.get_layout_as_text();
                                            if let Err(e) = f.write_all(&content.into_bytes()) {
                                                println!("{}", e);
                                            }
                                        } else {
                                            println!("Unable to create file");
                                        }
                                    } else {
                                        println!("Unable to retrieve reference to plot");
                                    }
                                },
                                _ => { println!("Layout extension should be .xml"); }
                            }
                        }
                    }
                },
                _ => { }
            }
        });
        {
            let xml_save_dialog = xml_save_dialog.clone();
            save_btn.connect_clicked(move |_btn| {
                xml_save_dialog.run();
                xml_save_dialog.hide();
            });
        }
        xml_save_dialog
    }

    pub fn new(
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
    ) -> LayoutWindow {
        let group_toolbar_top : Toolbar = builder.get_object("group_toolbar_top").unwrap();
        let group_toolbar_bottom : Toolbar = builder.get_object("group_toolbar_bottom").unwrap();
        let toolbars : [Toolbar; 2] = [group_toolbar_top.clone(), group_toolbar_bottom.clone()];

        let mut toggles = HashMap::new();
        let layout_iter = ALL_LAYOUTS.iter().zip(ALL_PATHS.iter());
        for (i, (layout, path)) in layout_iter.clone().enumerate() {
            let img = Image::from_file(&(String::from("assets/icons/") + path + ".svg"));
            let btn : ToggleToolButton = ToggleToolButton::new();
            btn.set_icon_widget(Some(&img));
            toggles.insert(*layout, btn.clone());
            toolbars[i / 4].insert(&btn, (i % 4) as i32);
        }
        toggles[&GroupSplit::Unique].set_active(true);
        group_toolbar_top.show_all();
        group_toolbar_bottom.show_all();

        for (layout, path) in layout_iter {
            let toggles = toggles.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let mapping_stack = mapping_stack.clone();
            toggles[layout].clone().connect_toggled(move |curr_toggle| {
                if curr_toggle.get_active() {
                    toggles.iter()
                        .filter(|(k, _)| *k != layout )
                        .for_each(|(_, btn)|{ btn.set_active(false) });
                    PlotWorkspace::clear_mappings(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                    ).expect("Error clearing mappings");
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        pl_view.update(&mut UpdateContent::Clear(format!("assets/plot_layout/{}.xml", path)));
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                } else {
                    // toggles[&GroupSplit::Unique].set_active(true);
                }
            });
        }

        let open_btn : Button = builder.get_object("layout_open_btn").unwrap();
        let save_btn : Button = builder.get_object("layout_save_btn").unwrap();
        let clear_btn : Button = builder.get_object("layout_clear_btn").unwrap();
        let query_btn : Button = builder.get_object("layout_query_btn").unwrap();
        let file_combo : ComboBoxText = builder.get_object("layout_file_combo").unwrap();
        let xml_load_dialog : FileChooserDialog = builder.get_object("xml_load_dialog").unwrap();
        let xml_save_dialog = Self::build_save_dialog(
            &builder,
            save_btn.clone(),
            file_combo.clone(),
            plot_view.clone()
        );

        /*{
            let xml_load_dialog = xml_load_dialog.clone();
            let plot_view = plot_view.clone();
            open_btn.connect_clicked(move |btn| {
                xml_load_dialog.show();
                xml_load_dialog.hide();
            });
        }*/

        {
            let xml_save_dialog = xml_save_dialog.clone();
            save_btn.connect_clicked(move |btn| {
                xml_save_dialog.show();
                xml_save_dialog.hide();
            });
        }

        LayoutWindow {
            toggles,
            open_btn,
            save_btn,
            clear_btn,
            query_btn,
            xml_save_dialog,
            xml_load_dialog,
            file_combo
        }
    }

    pub fn reset(&self, split : GroupSplit) {
        let toggle = &self.toggles[&split];
        if toggle.get_active() {
            toggle.set_active(false);
        } else {
            toggle.set_active(true);
        }
    }

    pub fn connect_layout_load_button(
        glade_def : Rc<HashMap<String, String>>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        // layout_clear_btn : ToolButton,
        plot_popover : PlotPopover,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_toggle : ToggleButton,
        // sidebar_stack : Stack,
        layout_window : LayoutWindow,
        layout_path : Rc<RefCell<Option<String>>>,
        xml_load_dialog : FileChooserDialog,
        load_btn : Button
    ) {
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            let plot_view = plot_view.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
                plot_view.borrow().parent.queue_draw();
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
                                            layout_window.reset(pl.group_split());
                                            *(layout_path.borrow_mut()) = Some(string_path);
                                            true
                                        },
                                        Err(e) => { println!("Unable to load layout: {}", e); false }
                                    }
                                },
                                Err(_) => { println!("Could not get mutable reference to Plot widget"); false }
                            };
                            if update_ok {
                                println!("Updating mapping widgets");
                                PlotWorkspace::update_mapping_widgets(
                                    plot_view.clone(),
                                    mapping_menus.clone(),
                                    //plot_notebook.clone(),
                                    plot_popover.clone(),
                                    glade_def.clone(),
                                    data_source.clone(),
                                    tbl_nb.clone(),
                                    status_stack.clone()
                                );
                                println!("Updating layout widgets");
                                Self::update_layout_widgets(
                                    design_menu.clone(),
                                    scale_menus.clone(),
                                    plot_view.clone()
                                );
                                println!("Layout widgets saved");
                                status_stack.try_show_alt();
                                plot_toggle.set_active(true);
                                // sidebar_stack.set_visible_child_name("layout");
                                // layout_clear_btn.set_sensitive(true);
                            } else {
                                println!("Failed at loadig layout. Widgets will not be updated");
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
    }

    fn update_layout_widgets(
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let (design, info_x, info_y) = match plot_view.try_borrow() {
            Ok(pl) => {
                let design = pl.plot_group.design_info();
                let info_x = pl.current_scale_info("x");
                let info_y = pl.current_scale_info("y");
                (design, info_x, info_y)
            },
            _ => {
                println!("Could not fetch plotview reference to update layout");
                return;
            }
        };

        // It is important to call those updates outside the plot_view borrow because those updates
        // will trigger the scale_set, entry_set, etc. signals inside each menu, which
        // assume plot_view can be borrowed mutably.
        design_menu.update(design);
        scale_menus.0.update(info_x);
        scale_menus.1.update(info_y);
    }

}

