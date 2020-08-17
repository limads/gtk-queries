use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell};
use std::fs::File;
use std::io::Read;
use gdk::{self, keys};
use crate::tables::{environment::TableEnvironment, environment::EnvironmentUpdate};
use sourceview::*;
use gtk::prelude::*;
use crate::{status_stack::StatusStack };
use crate::status_stack::*;
use sourceview::View;
use super::sql_editor::SqlEditor;

#[derive(Clone, Debug)]
pub struct MainMenu {
    main_menu : PopoverMenu,
    main_toggle : ToggleButton,
    engine_btn : ModelButton,
    sql_open_btn : ModelButton,
    engine_window : Window,
    settings_btn : ModelButton,
    sql_new_btn : ModelButton,
    settings_window : Window,
    layout_window : Window,
    layout_btn : ModelButton
    // library_list_box : ListBox,
    // fn_list_box : ListBox
}

impl MainMenu {

    fn link_window(btn : ModelButton, win : Window) {
        {
        let win = win.clone();
            btn.connect_clicked(move |_| {
                win.show();
            });
        }
        win.set_destroy_with_parent(false);
        win.connect_delete_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });
        win.connect_destroy_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });
    }

    pub fn build(builder : &Builder, sql_editor : &SqlEditor, content_stack : Stack, query_toggle : ToggleButton) -> Self {
        let main_menu : PopoverMenu = builder.get_object("main_menu").unwrap();
        let sql_new_btn : ModelButton = builder.get_object("sql_new_btn").unwrap();
        let main_toggle : ToggleButton = builder.get_object("main_toggle").unwrap();
        let engine_btn : ModelButton = builder.get_object("engine_btn").unwrap();
        let layout_btn : ModelButton = builder.get_object("layout_btn").unwrap();
        let settings_btn : ModelButton = builder.get_object("settings_btn").unwrap();
        let engine_window : Window = builder.get_object("engine_window").unwrap();
        let settings_window : Window = builder.get_object("settings_window").unwrap();
        let layout_window : Window = builder.get_object("layout_window").unwrap();
        // engine_window.set_destroy_with_parent(false);
        // settings_window.set_destroy_with_parent(false);
        Self::link_window(engine_btn.clone(), engine_window.clone());
        Self::link_window(settings_btn.clone(), settings_window.clone());
        Self::link_window(layout_btn.clone(), layout_window.clone());
        {
            let main_menu = main_menu.clone();
            main_toggle.connect_toggled(move |btn| {
                if btn.get_active() {
                    main_menu.show();
                } else {
                    main_menu.hide();
                }
            });
        }

        {
            let main_toggle = main_toggle.clone();
            main_menu.connect_closed(move |_| {
                if main_toggle.get_active() {
                    main_toggle.set_active(false);
                }
            });
        }

        let sql_open_btn : ModelButton = builder.get_object("sql_open_btn").unwrap();
        {
            let sql_editor = sql_editor.clone();
            sql_open_btn.connect_clicked(move |btn| {
                sql_editor.sql_load_dialog.run();
                //sql_editor.sql_load_dialog.hide();
            });
        }

        {
            let sql_editor = sql_editor.clone();
            let content_stack = content_stack.clone();
            let query_toggle = query_toggle.clone();
            sql_new_btn.connect_clicked(move |btn|{
                sql_editor.add_fresh_editor(content_stack.clone(), query_toggle.clone());
            });
        }

        MainMenu {
            main_menu,
            main_toggle,
            engine_btn,
            engine_window,
            settings_btn,
            settings_window,
            layout_window,
            layout_btn,
            sql_open_btn,
            sql_new_btn
        }
    }

}


