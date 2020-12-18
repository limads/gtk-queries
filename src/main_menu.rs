use gtk::*;
// use gio::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
// use std::fs::File;
// use std::io::Read;
// use gdk;
use crate::tables::environment::TableEnvironment;
// use sourceview::*;
use gtk::prelude::*;
// use crate::{status_stack::StatusStack };
// use crate::status_stack::*;
// use sourceview::View;
use super::sql_editor::SqlEditor;
// use std::io::Write;
use crate::table_notebook::TableNotebook;
use crate::plots::plotview::plot_view::PlotView;
use crate::utils;
use crate::report;
use crate::command::CommandWindow;

#[derive(Clone, Debug)]
pub struct MainMenu {
    main_menu : PopoverMenu,
    main_toggle : ToggleButton,
    // engine_btn : ModelButton,
    sql_open_btn : ModelButton,
    // engine_window : Window,
    settings_btn : ModelButton,
    sql_new_btn : ModelButton,
    sql_save_btn : ModelButton,
    menu_run_btn : ModelButton,
    // jobs_btn : ModelButton,
    // jobs_window : Window,
    settings_window : Window,
    pub layout_window : Window,
    pub cmd_window : CommandWindow,
    report_window : Window,
    report_btn : ModelButton,
    layout_btn : ModelButton,
    save_img_btn : ModelButton,
    // save_tbl_btn : ModelButton,
    // sql_open_dialog : FileChooserDialog,
    // library_list_box : ListBox,
    // fn_list_box : ListBox
}

impl MainMenu {

    fn build_save_image_btn(
        builder : &Builder,
        pl_view : Rc<RefCell<PlotView>>
    ) -> ModelButton {
        let save_image_btn : ModelButton =
            builder.get_object("save_image_btn").unwrap();
        let save_img_dialog : FileChooserDialog =
            builder.get_object("save_img_dialog").unwrap();

        {
            let save_img_dialog = save_img_dialog.clone();
            save_image_btn.connect_clicked(move |_btn| {
                save_img_dialog.run();
                save_img_dialog.hide();
            });
        }

        save_img_dialog.connect_response(move |dialog, resp|{
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        if let Ok(mut pl) = pl_view.try_borrow_mut() {
                            if let Some(p) = path.to_str() {
                                if let Err(e) = pl.plot_group.draw_to_file(p) {
                                    println!("{}", e);
                                }
                            } else {
                                println!("Could not retrieve path as str");
                            }
                        } else {
                            println!("Could not retrieve reference to pl_view when saving image");
                        }
                    } else {
                        println!("Invalid path for image");
                    }
                },
                _ => { }
            }
        });
        save_image_btn
    }

    /*fn build_sql_open_dialog(builder : &Builder, editor : &SqlEditor) -> FileChooserDialog {
        let sql_open_dialog FileChooserDialog =
            builder.get_object("sql_open_dialog").unwrap();

        sql_open_dialog
    }*/

    pub fn build(
        builder : &Builder,
        sql_editor : &SqlEditor,
        content_stack : Stack,
        query_toggle : ToggleButton,
        view : Rc<RefCell<PlotView>>,
        tbl_nb : TableNotebook,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        editor : SqlEditor
    ) -> Self {
        let main_menu : PopoverMenu = builder.get_object("main_menu").unwrap();
        let sql_new_btn : ModelButton = builder.get_object("sql_new_btn").unwrap();
        let main_toggle : ToggleButton = builder.get_object("main_toggle").unwrap();
        // let engine_btn : ModelButton = builder.get_object("engine_btn").unwrap();
        let layout_btn : ModelButton = builder.get_object("layout_btn").unwrap();
        let settings_btn : ModelButton = builder.get_object("settings_btn").unwrap();
        let menu_run_btn : ModelButton = builder.get_object("menu_run_btn").unwrap();
        // let engine_window : Window = builder.get_object("engine_window").unwrap();
        let report_btn : ModelButton = builder.get_object("report_btn").unwrap();
        let report_window : Window = builder.get_object("report_window").unwrap();
        let cmd_window = CommandWindow::build(&builder, &tbl_nb, tbl_env.clone());
        
        // Build report window
        let report_template_btn : FileChooserButton = builder.get_object("report_template_btn").unwrap();
        let report_save_btn : Button = builder.get_object("report_save_btn").unwrap();
        let report_save_win : FileChooserDialog = builder.get_object("report_save_window").unwrap();
        {
            let report_save_win = report_save_win.clone();
            report_save_btn.connect_clicked(move |_btn| {
                report_save_win.run();
                report_save_win.hide();
            });
        }
        
        report_save_win.connect_response(move |dialog, resp| {
            match resp {
                ResponseType::Other(1) => {
                    if let Some(templ_path) = report_template_btn.get_filename().and_then(|t| t.to_str().map(|t| t.to_string())) {
                        if let Some(out_path) = dialog.get_filename().and_then(|t| t.to_str().map(|t| t.to_string()) ) {
                            let rep_ans = report::write_report(
                                &templ_path[..], 
                                &out_path[..],
                                tbl_env.clone()
                            );
                            if let Err(e) = rep_ans {
                                println!("{}", e);
                            }
                        } else {
                            println!("Missing report out path");
                        }
                    } else {
                        println!("Missing report template path");
                    }
                },
                _ => { }
            }
        });
        
        //let jobs_window : Window = builder.get_object("jobs_window").unwrap();
        let settings_window : Window = builder.get_object("settings_window").unwrap();
        let layout_window : Window = builder.get_object("layout_window").unwrap();
        let sql_save_btn : ModelButton = builder.get_object("sql_save_btn").unwrap();
        let sql_open_btn : ModelButton = builder.get_object("sql_open_btn").unwrap();
        // engine_window.set_destroy_with_parent(false);
        // settings_window.set_destroy_with_parent(false);
        // utils::link_window(engine_btn.clone(), engine_window.clone());
        utils::link_window(settings_btn.clone(), settings_window.clone());
        utils::link_window(layout_btn.clone(), layout_window.clone());
        utils::link_window(report_btn.clone(), report_window.clone());
        utils::link_window(menu_run_btn.clone(), cmd_window.win.clone());

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
        /*{
            let sql_editor = sql_editor.clone();
            sql_open_btn.connect_clicked(move |btn| {
                sql_editor.sql_load_dialog.run();
                //sql_editor.sql_load_dialog.hide();
            });
        }*/

        {
            let sql_editor = sql_editor.clone();
            let content_stack = content_stack.clone();
            let query_toggle = query_toggle.clone();
            sql_new_btn.connect_clicked(move |_btn|{
                sql_editor.add_fresh_editor(content_stack.clone(), query_toggle.clone());
            });
        }

        let save_img_btn = Self::build_save_image_btn(&builder, view);
        // let save_tbl_btn = Self::build_save_table_btn(&builder, tbl_nb, tbl_env);
        // let sql_open_dialog = Self::build_sql_open_dialog(&builder, &editor);
        // let sql_save_dialog = Self::build_sql_save_dialog(&builder, &editor);

        {
            let sql_load_dialog = sql_editor.sql_load_dialog.clone();
            sql_open_btn.connect_clicked(move |_btn| {
                sql_load_dialog.run();
                sql_load_dialog.hide();
            });
        }

        {
            // let sql_save_dialog = sql_editor.sql_save_dialog.clone();
            let sql_editor = sql_editor.clone();
            sql_save_btn.connect_clicked(move |_btn| {
                sql_editor.save_current();
            });
        }
        
        MainMenu {
            main_menu,
            main_toggle,
            menu_run_btn,
            // engine_btn,
            // engine_window,
            settings_btn,
            settings_window,
            layout_window,
            layout_btn,
            sql_open_btn,
            sql_new_btn,
            sql_save_btn,
            save_img_btn,
            // save_tbl_btn,
            report_window,
            report_btn,
            // menu_run_btn,
            cmd_window
            // jobs_btn,
            //jobs_window
            // sql_open_dialog,
            // sql_save_dialog,
        }
    }

}

/*let xml_save_dialog : FileChooserDialog =
builder.get_object("xml_save_dialog").unwrap();
{
    let rc_view = pl_view.clone();
    xml_save_dialog.connect_response(move |dialog, resp|{
        // println!("{:?}", resp);
        match resp {
            ResponseType::Other(1) => {
                if let Some(path) = dialog.get_filename() {
                    if let Ok(pl) = rc_view.try_borrow() {
                        if let Ok(mut f) = File::create(path) {
                            let content = pl.plot_area.get_layout_as_text();
                            let _ = f.write_all(&content.into_bytes());
                        }
                    }
                }
            },
            _ => { }
        }
    });
}
let save_layout_btn : Button =
    builder.get_object("save_layout_btn").unwrap();
let xml_save_rc = xml_save_dialog.clone();
save_layout_btn.connect_clicked(move |_| {
    xml_save_rc.run();
    xml_save_rc.hide();
});*/
