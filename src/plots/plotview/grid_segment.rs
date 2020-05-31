use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct GridSegment {
    pub label : String,
    pub precision : i32,
    pub from : f64,
    pub to : f64,
    pub steps : Vec<f64>,
    pub log : bool,
    pub invert : bool,
    pub offset : i32,
}

impl GridSegment {

    pub fn new(
        label : String, precision : i32, from : f64, to : f64,
        n_intervals : i32, log : bool, invert : bool, offset : i32
    ) -> GridSegment {

        let off_prop = ((to - from) / n_intervals as f64)*(offset as f64 / 100.0);
        let from_offset = from + off_prop;

        /*let intv_size = match log {
            true => (to.log10() - from.log10()) / (n_intervals as f64),
            false => (to - from) / (n_intervals as f64)
        };*/

        let intv_size = match log {
            true => (to.log10() - from.log10() /*- 2.0*off_prop.log10() */ ) / (n_intervals as f64),
            false => (to - from /*- 2.0*off_prop */ ) / (n_intervals as f64)
        };

        let mut steps = Vec::<f64>::new();
        let base : f64 = 10.0;
        for i in 0..n_intervals+1 {
            let step = if log {
                base.powf(from_offset.log10() + (i as f64)*intv_size)
            } else {
                from_offset + (i as f64)*intv_size
            };
            steps.push(step);
        }
        GridSegment{label, precision, from, to, steps, log, invert, offset}
    }

    pub fn description(&self) -> HashMap<String, String> {
        let mut desc = HashMap::new();
        desc.insert("label".into(), self.label.clone());
        desc.insert("precision".into(), self.precision.to_string());
        desc.insert("from".into(), self.from.to_string());
        desc.insert("to".into(), self.to.to_string());
        desc.insert("n_intervals".into(), self.steps.len().to_string());
        desc.insert("invert".into(), self.invert.to_string());
        desc.insert("log_scaling".into(), self.log.to_string());
        desc.insert("grid_offset".into(), self.offset.to_string());
        desc.insert("grid_density".into(), self.steps.len().to_string());
        desc
    }

}

