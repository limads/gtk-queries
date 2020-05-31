use super::*;

pub struct PlotView {
    pub plot_group : PlotGroup,
    //pub plot_area : PlotArea,
    parent : gtk::DrawingArea,
    active_area : usize,
    layout_path : String
}

pub enum UpdateContent {

    /// Layout path, Layout value
    Layout(String, String),

    /// Mapping name, position values
    Data(String, Vec<Vec<f64>>),

    /// Mapping name, position values, text values
    TextData(String, Vec<Vec<f64>>, Vec<String>),

    /// Mapping name; Mapping type
    NewMapping(String, String),

    // Mapping id; Column id; Column names;
    //MappingColumn(String, Vec<String>),

    /// Mapping id
    RemoveMapping(String),

    /// Mapping id; New id; New type.
    EditMapping(String, String, String),

    // Pass (old, new) mapping name
    // RenameMapping(String, String),

    /// Clears all data and displays layout at the informed path
    Clear(String),
}

impl PlotView {

    pub fn redraw(&self) {
        self.parent.queue_draw();
    }

    /* Starts a new PlotView with an enclosed DrawingArea */
    pub fn new(layout_path : &str) -> Rc<RefCell<PlotView>> {
        let draw_area = gtk::DrawingArea::new();
        PlotView::new_with_draw_area(layout_path, draw_area)
    }

    pub fn group_split(&self) -> GroupSplit {
        self.plot_group.group_split()
    }

    pub fn change_active_area(&mut self, area : usize) {
        self.active_area = area;
    }

    pub fn get_active_area(&self) -> usize {
        self.active_area
    }

    /* If you want to add the PlotDrawing behavior to an
    already instantiated draw area (i.e. built from glade) */
    pub fn new_with_draw_area(
        layout_path : &str,
        draw_area : gtk::DrawingArea,
    ) -> Rc<RefCell<PlotView>> {
        let plot_group = PlotGroup::new(String::from(layout_path)).unwrap();
        let plot_view = Rc::new(RefCell::new(
            PlotView{plot_group, parent : draw_area, active_area : 0, layout_path : layout_path.into() }));
        let plot_ref = Rc::clone(&plot_view);
        if let Ok (pl_mut) = plot_ref.try_borrow_mut() {
            let plot_ref = Rc::clone(&plot_view);
            pl_mut.parent.connect_draw(move |da, ctx| {
                let allocation = da.get_allocation();
                let w = allocation.width;
                let h = allocation.height;
                if let Ok(mut pl) = plot_ref.try_borrow_mut() {
                    pl.plot_group.draw_to_context(&ctx, 0, 0, w, h);
                }
                glib::signal::Inhibit(true)
            });
        } else {
            println!("Error in getting mutable reference to plot_group");
        }
        plot_view
    }

    pub fn current_scale_info(&self, scale : &str) -> HashMap<String, String> {
        self.plot_group.scale_info(self.active_area, scale)
    }

    pub fn mapping_info(&self) -> Vec<(String, String, HashMap<String,String>)> {
        self.plot_group.mapping_info(self.active_area)
    }

    fn insert_mapping(&mut self, ix : usize, m_name : String, m_type : String) {
        let maybe_update = self.plot_group.add_mapping(
            ix,
            m_name.to_string(),
            m_type.to_string()
        );
        if let Err(e) = maybe_update {
            println!("Error adding new mapping: {}", e);
        }
    }

    pub fn update(&mut self, content : &mut UpdateContent) {

        //if let Ok(mut ref_area) = self.plot_area.try_borrow_mut() {
        let active = self.active_area;
        match content {
            UpdateContent::Layout(key, property) => {
                self.plot_group.update_layout(active, &key, &property);
                /*if self.plot_area.reload_layout_data().is_err() {
                    println!(
                        "Error updating property {:?} with value {:?}",
                            key, property);
                }*/
                self.parent.queue_draw();
            },
            UpdateContent::Data(key, data) => {
                if let Err(e) = self.plot_group.update_mapping(active, &key, data) {
                    println!("Error updating mapping {:}: {}", key, e);
                }
                self.parent.queue_draw();
            },
            UpdateContent::TextData(key, pos, text) => {
                match self.plot_group.update_mapping(active, &key, pos) {
                    Err(e) => { println!("Error updating text mapping: {}", e); },
                    _ => {
                        if let Err(e) = self.plot_group.update_mapping_text(active, &key, text) {
                            println!("Error adding text to mapping: {}", e);
                        }
                    }
                }
                self.parent.queue_draw();
            },
            //UpdateContent::MappingColumn(m_name, cols) => {
            //    panic!("Unimplemented")
                /*let old_cols = self.plot_area.mapping_column_names(m_name);
                for ((col, _), new_name) in old_cols.iter().zip(cols.iter()) {
                    let prop_name = String::from("/gridplot/object[@name='") +
                        &m_name + "']/property[@name='" + &col + "']";
                    self.plot_area.update_layout(&prop_name, new_name);
                }*/
            //},
            UpdateContent::NewMapping(m_name, m_type) => {
                self.insert_mapping(active, m_name.clone(), m_type.clone());
                self.parent.queue_draw();
            },
            UpdateContent::EditMapping(m_name, new_name, new_type) => {
                self.plot_group.remove_mapping(active, m_name);
                self.insert_mapping(active, new_name.clone(), new_type.clone());
                self.parent.queue_draw();
            },
            UpdateContent::RemoveMapping(m_name) => {
                self.plot_group.remove_mapping(active, m_name);
                self.parent.queue_draw();
            },
            //UpdateContent::RenameMapping(old, new) => {
            //},
            UpdateContent::Clear(path) => {
                if let Err(e) = self.plot_group.load_layout(path.clone()) {
                    println!("{}", e);
                } else {
                    self.layout_path = path.to_string();
                }
                self.parent.queue_draw();
            }
        }

        //} else {
        //    println!("Could not get mutable reference to plot area");
        //}
    }
}

