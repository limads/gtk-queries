use gtk::*;
use gtk::prelude::*;
// use gio::prelude::*;
use super::mapping_menu::*;
//use crate::PlotSidebar;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::{source::EnvironmentSource, environment::TableEnvironment};
use gtkplotview::plot_view::{PlotView, UpdateContent};
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use gio::FileExt;
use super::design_menu::*;
use super::scale_menu::*;
use std::collections::HashMap;
use crate::utils;
use crate::status_stack::StatusStack;
use crate::table_notebook::TableNotebook;

/// PlotsSidebar holds the information of the used mappings
#[derive(Clone)]
pub struct PlotSidebar {
    pub design_menu : DesignMenu,
    pub scale_menus : (ScaleMenu, ScaleMenu),
    pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    pub notebook : Notebook,
    pub layout_stack : Stack,
    //pub sidebar_box : Box
}

impl PlotSidebar {

    pub fn layout_loaded(&self) -> bool {
        let sel_name = self.layout_stack.get_visible_child_name()
            .map(|n| n.to_string()).unwrap_or(String::from("empty"));
        match &sel_name[..] {
            "layout" => true,
            _ => false
        }
    }

    pub fn new(
        builder : Builder,
        pl_view : Rc<RefCell<PlotView>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack
    ) -> Self {
        //let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let mapping_menus : Vec<MappingMenu> = Vec::new();
        let mapping_menus = Rc::new(RefCell::new(mapping_menus));
        let design_menu = build_design_menu(&builder, pl_view.clone());
        let plot_notebook : Notebook =
            builder.get_object("plot_notebook").unwrap();
        let scale_menus = build_scale_menus(&builder, pl_view.clone());
        //let sidebar_box : Box = builder.get_object("sidebar_box").unwrap();
        let layout_stack : Stack = builder.get_object("layout_stack").unwrap();
        let sidebar = PlotSidebar {
            design_menu,
            scale_menus,
            mapping_menus,
            notebook : plot_notebook.clone(),
            layout_stack : layout_stack.clone(),
        //    sidebar_box
        };
        let _layout_menu = LayoutMenu::new_from_builder(
            &builder,
            pl_view.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            status_stack,
            sidebar.clone()
        );
        sidebar
    }

    /*pub fn used_names_and_types(&self) -> (Vec<String>, Vec<String>) {
        let mut names = Vec::new();
        let mut types = Vec::new();
        println!("Ref Count: {}", Rc::strong_count(&self.mapping_menus));
        if let Ok(_) = self.mapping_menus.try_borrow() {
            println!("Could borrow at used_names_and_types");
        } else {
            println!("Could not borrow at used_names_and_types");
        }

        match self.mapping_menus.try_borrow() {
            Ok(m_menus) =>  {
                for m in m_menus.iter() {
                    names.push(m.get_mapping_name());
                    types.push(m.mapping_type.clone());
                }
            },
            Err(e) => println!("Could not retrieve reference over mapping menus: {}", e),
        }
        (names, types)
    }*/

    /*pub fn update_info(&mut self, ix : usize, new_name : String, new_type : String) {
        if let Ok(mut m_menus) = self.mapping_menus.try_borrow_mut() {
            if let Some(m) = m_menus.get_mut(ix) {

            } else {
                println!("Unable to retrieve mapping menu at index");
            }
        } else {
            println!("Could not recover reference to mapping menus");
        }
    }*/

}

/// LayoutMenu encapsulate the logic of the buttons at the bottom-left
/// that allows changing the plot layout and mappings.
#[derive(Clone)]
pub struct LayoutMenu {
    load_layout_btn : Button,
    new_layout_btn : Button,
    add_mapping_btn : ToolButton,
    //manage_btn : Button,
    remove_mapping_btn : ToolButton,
    layout_stack : Stack
    //manage_mapping_popover : Popover
}

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

impl LayoutMenu {

    /// The creation of a mapping menu is based on an id naming convention
    /// of passing a prefix identifying the mappping (line, scatter, box, etc)
    /// followed by an element identifier. This convention applies to the enclosing box
    /// (line_box, scatter_box ...) and its constituint widgets (scatter_color_button,
    /// line_color_button) and so on. The builder for each mapping menu must be unique
    /// to avoid aliasing.
    /// Make this mapping_menu::create(.)
    fn create_new_mapping_menu(
        builder : Builder,
        mapping_name : String,
        mapping_type : String,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        pl_view : Rc<RefCell<PlotView>>,
        properties : Option<HashMap<String, String>>,
        sidebar : PlotSidebar
    ) -> Result<MappingMenu, &'static str> {
        let valid_mappings = ["line", "scatter", "bar", "area", "text", "surface"];
        if !valid_mappings.iter().any(|s| &mapping_type[..] == *s) {
            return Err("Invalid mapping type. Must be line|scatter|bar|area|text|surface");
        }
        let box_name = mapping_type.clone() + "_box";
        let mapping_box : Box = builder.get_object(&box_name).unwrap();
        let design_widgets = HashMap::new();
        let mut m = MappingMenu {
            mapping_name,
            mapping_type,
            mapping_box,
            design_widgets,
        };
        m.build_mapping_design_widgets(
            &builder,
            pl_view.clone()
        );

        if let Some(prop) = properties {
            if let Err(e) = m.update_widget_values(prop) {
                println!("{}", e);
            }
        }
        Ok(m)
    }

    fn create_tab_image(m_type : String) -> Image {
        let tab_img_path = String::from("assets/icons/") + &m_type + ".svg";
        Image::new_from_file(&tab_img_path[..])
    }

    fn append_mapping_menu(
        mut m : MappingMenu,
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        notebook : Notebook,
        plot_view : Rc<RefCell<PlotView>>,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        pos : Option<usize>
    ) {
        match (plot_view.try_borrow_mut(), tbl_env.try_borrow(), mappings.try_borrow_mut()) {
            (Ok(mut pl), Ok(t_env), Ok(mut mappings)) => {
                //m.update_available_cols(source.col_names(), &pl);
                match pos {
                    Some(p) => mappings.insert(p, m.clone()),
                    None => mappings.push(m.clone())
                }
                let tab_img = Self::create_tab_image(m.mapping_type.clone());
                notebook.add(&m.get_parent());
                notebook.set_tab_label(&m.get_parent(), Some(&tab_img));
                let npages = notebook.get_children().len() as i32;
                notebook.set_property_page(npages-1);
                notebook.show_all();
                pl.update(&mut UpdateContent::NewMapping(
                    m.mapping_name.to_string(),
                    m.mapping_type.to_string())
                );
                let selected = tbl_nb.full_selected_cols();
                let cols = t_env.get_columns(&selected[..]);
                println!("{:?}", cols);
                if let Err(e) = m.update_data(cols, &mut pl) {
                    println!("{}", e);
                }
            },
            (_,_,Err(e)) => { println!("{}", e); },
            _ => {
                println!("Unable to retrieve mutable reference to plot view|data source");
            }
        }
    }

    fn clear_mappings(
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        plot_notebook : Notebook
    ) -> Result<(), &'static str> {
        if let Ok(mut mappings) = mappings.try_borrow_mut() {
            for m in mappings.iter() {
                plot_notebook.remove(&m.get_parent());
            }
            mappings.clear();
            Ok(())
        } else {
            Err("Could not fetch mutable reference to mapping menus before clearing them")
        }
    }

    fn update_layout_widgets(
        sidebar : PlotSidebar,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        match plot_view.try_borrow_mut() {
            Ok(pl) => {
                sidebar.design_menu.update(pl.plot_area.design_info());
                sidebar.scale_menus.0.update(pl.plot_area.scale_info("x"));
                sidebar.scale_menus.1.update(pl.plot_area.scale_info("y"));
            },
            _ => {
                panic!("Could not fetch plotview reference to update layout");
            }
        }
    }

    fn build_layout_load_button(
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
                                    builder.clone(),
                                    m_info.0.clone(),
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
                            sidebar.layout_stack.set_visible_child_name("layout");
                            //println!("{:?}", mappings);
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
        load_btn
    }

    fn selected_mapping_radio(scatter_radio : &RadioButton) -> Option<String> {
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
    }

    fn remove_selected_mapping_page(
        plot_notebook : &Notebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let page = plot_notebook.get_property_page() as usize;
        let mapping_ix = page - 3;
        let children = plot_notebook.get_children();
        if let Some(c) = children.get(page) {
            if let Ok(mut menus) = mapping_menus.try_borrow_mut() {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    menus.remove(mapping_ix);
                    plot_notebook.remove(c);
                    let name = (mapping_ix).to_string();
                    pl_view.update(&mut UpdateContent::RemoveMapping(name));
                } else {
                    println!("Could not get mutable reference to PlotView")
                }
            } else {
                println!("Unable to retrieve mutable reference to mapping_menus when removing page");
            }
        } else {
            println!("Invalid child position");
        }
    }

    /// Add mapping from a type string description, attributing to its
    /// name the number of mappings currently used.
    pub fn add_mapping_from_type(
        mapping_type : &str,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        plot_view : Rc<RefCell<PlotView>>,
        sidebar : PlotSidebar,
        builder_clone : Builder
    ) {
        let name = if let Ok(menus) = sidebar.mapping_menus.try_borrow() {
            format!("{}", menus.len())
        } else {
            return;
        };
        let menu = LayoutMenu::create_new_mapping_menu(
            builder_clone.clone(),
            name,
            mapping_type.to_string(),
            data_source.clone(),
            plot_view.clone(),
            None,
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

    pub fn new_from_builder(
        builder : &Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        sidebar : PlotSidebar
    ) -> Self {

        let layout_toolbar : Toolbar = builder.get_object("layout_toolbar").unwrap();
        let img_add = Image::new_from_icon_name(Some("list-add-symbolic"), IconSize::SmallToolbar);
        let img_remove = Image::new_from_icon_name(Some("list-remove-symbolic"), IconSize::SmallToolbar);
        let img_clear = Image::new_from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let clear_layout_btn : ToolButton = ToolButton::new(Some(&img_clear), None);
        let add_mapping_btn : ToolButton = ToolButton::new(Some(&img_add), None);
        let remove_mapping_btn : ToolButton = ToolButton::new(Some(&img_remove), None);
        remove_mapping_btn.set_sensitive(false);
        clear_layout_btn.set_sensitive(false);
        layout_toolbar.insert(&clear_layout_btn, 0);
        layout_toolbar.insert(&add_mapping_btn, 1);
        layout_toolbar.insert(&remove_mapping_btn, 2);
        layout_toolbar.show_all();

        let load_layout_btn = Self::build_layout_load_button(
            builder.clone(),
            plot_view.clone(),
            data_source.clone(),
            sidebar.clone(),
            tbl_nb.clone(),
            status_stack.clone(),
            clear_layout_btn.clone()
        );

        let new_layout_btn : Button = builder.get_object("layout_new_btn").unwrap();
        let layout_stack = sidebar.layout_stack.clone();
        {
            let layout_stack = layout_stack.clone();
            let status_stack = status_stack.clone();
            let clear_layout_btn = clear_layout_btn.clone();
            new_layout_btn.connect_clicked(move |btn| {
                layout_stack.set_visible_child_name("layout");
                status_stack.try_show_alt();
                clear_layout_btn.set_sensitive(true);
            });
        }

        {
            let layout_stack = layout_stack.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = sidebar.mapping_menus.clone();
            let notebook = sidebar.notebook.clone();
            clear_layout_btn.connect_clicked(move |btn| {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout.xml")));
                    layout_stack.set_visible_child_name("empty");
                    status_stack.show_curr_status();
                    if let Ok(mut mappings) = mapping_menus.try_borrow_mut() {
                        mappings.clear();
                    } else {
                        println!("Error retrieving mapping menus");
                    }
                    let children = notebook.get_children();
                    for i in 3..children.len() {
                        if let Some(c) = children.get(i) {
                            notebook.remove(c);
                        } else {
                            println!("Unable to clear notebook");
                        }
                    }
                }
                btn.set_sensitive(false);
            });
        }

        let add_mapping_popover : Popover = builder.get_object("add_mapping_popover").unwrap();
        add_mapping_popover.set_relative_to(Some(&add_mapping_btn));
        let upper_mapping_toolbar : Toolbar = builder.get_object("upper_mapping_toolbar").unwrap();
        let lower_mapping_toolbar : Toolbar = builder.get_object("lower_mapping_toolbar").unwrap();
        let toolbars = [upper_mapping_toolbar, lower_mapping_toolbar];
        let mapping_names = vec![
            String::from("line"),
            String::from("scatter"),
            String::from("bar"),
            String::from("text"),
            String::from("area"),
            String::from("surface")
        ];
        let iter_names = mapping_names.iter();
        for (i, mapping) in iter_names.enumerate() {
            //let mut m_name = String::from(&mapping[0..1].to_uppercase());
            //m_name += &mapping[1..];
            let img = Image::new_from_file(&(String::from("assets/icons/") +  mapping + ".svg"));
            let btn : ToolButton = ToolButton::new(Some(&img), None);
            toolbars[i / 3].insert(&btn, (i % 3) as i32);
            let m = mapping.clone();
            let add_mapping_popover = add_mapping_popover.clone();
            let builder = builder.clone();
            let data_source = data_source.clone();
            let plot_view = plot_view.clone();
            let sidebar = sidebar.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            let tbl_nb = tbl_nb.clone();
            btn.connect_clicked(move |_btn| {
                Self::add_mapping_from_type(
                    &m[..],
                    data_source.clone(),
                    tbl_nb.clone(),
                    plot_view.clone(),
                    sidebar.clone(),
                    builder.clone()
                );
                add_mapping_popover.hide();
                remove_mapping_btn.set_sensitive(true);
            });
        }
        toolbars.iter().for_each(|t| t.show_all() );
        add_mapping_btn.connect_clicked(move|_btn| {
            add_mapping_popover.show();
        });

        {
            let plot_notebook = sidebar.notebook.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            plot_notebook.clone().connect_switch_page(move |_nb, wid, page| {
                //let page = plot_notebook.get_property_page();
                println!("{}", page);
                if page > 2 {
                    remove_mapping_btn.set_sensitive(true);
                } else {
                    remove_mapping_btn.set_sensitive(false);
                }
            });
        }

        {
            let sidebar = sidebar.clone();
            let plot_view = plot_view.clone();
            let plot_notebook = sidebar.notebook.clone();
            remove_mapping_btn.connect_clicked(move |_| {
                    Self::remove_selected_mapping_page(
                        &plot_notebook,
                        sidebar.mapping_menus.clone(),
                        plot_view.clone()
                    );
                plot_notebook.show_all();
            });
        }

        Self {
            load_layout_btn,
            add_mapping_btn,
            new_layout_btn,
            remove_mapping_btn,
            layout_stack
            //manage_mapping_popover
        }
    }

}


