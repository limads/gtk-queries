use cairo::Context;
use cairo::*;
use cairo::ScaledFont;
use super::context_mapper::*;
use std::f64::consts::PI;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct FontData {
    pub font_family : String,
    pub font_weight : FontWeight,
    pub font_slant : FontSlant,
    pub font_size : i32,
    pub sf : ScaledFont
}

impl FontData {

    pub fn create_standard_font() -> Self {
        let font_family = String::from("Liberation Sans");
        let font_weight = FontWeight::Normal;
        let font_slant = FontSlant::Normal;
        let font_size = 12;
        let sf = create_scaled_font(
            &font_family[..],
            font_slant,
            font_weight,
            font_size
        );
        Self{ font_family, font_weight, font_slant, font_size, sf }
    }

    pub fn new_from_string(font : &str) -> Self {
        let digits_pattern = Regex::new(r"\d{2}$|\d{2}$").unwrap();
        let sz_match = digits_pattern.find(&font).expect("No font size");
        let sz_txt = sz_match.as_str();
        let font_size = sz_txt.parse().expect("Unnable to parse font");
        let mut prefix = &font[0..sz_match.start()];
        let slant_pattern = Regex::new("Italic|Oblique").unwrap();
        let slant_match = slant_pattern.find(prefix);
        let font_slant = match slant_match {
            Some(m) => {
                match m.as_str() {
                    "Italic" => FontSlant::Italic,
                    "Oblique" => FontSlant::Oblique,
                    _ => FontSlant::Normal
                }
            },
            None => FontSlant::Normal
        };
        if let Some(slant) = slant_match {
            prefix = &font[0..slant.start()];
        };
        let weight_pattern = Regex::new("Bold").unwrap();
        let weight_match = weight_pattern.find(prefix);
        let font_weight = match weight_match {
            Some(_w) => FontWeight::Bold,
            None => FontWeight::Normal
        };
        if let Some(weight) = weight_match {
            prefix = &font[0..weight.start()];
        };
        let font_family = String::from(prefix);
        let sf = create_scaled_font(
            &font_family[..],
            font_slant,
            font_weight,
            font_size
        );
        Self {
            font_family,
            font_weight,
            font_slant,
            font_size,
            sf
        }
    }

    pub fn description(&self) -> String {
        let mut font = self.font_family.clone();
        font = font + match self.font_slant {
            FontSlant::Normal => "",
            FontSlant::Oblique => " Oblique",
            FontSlant::Italic => " Italic",
            _ => ""
        };
        font = font + match self.font_weight {
            FontWeight::Normal => "",
            FontWeight::Bold => " Bold",
            _ => ""
        };
        font = font + &self.font_size.to_string();
        font
    }

    pub fn set_font_into_context(&self, ctx : &Context) {
        ctx.set_font_size(self.font_size as f64);
        ctx.select_font_face(
            &self.font_family,
            self.font_slant,
            self.font_weight
        );
    }
}

fn create_scaled_font(
    family : &str,
    slant : FontSlant,
    weight : FontWeight,
    size : i32
) -> ScaledFont {
    let mut font_m =  Matrix::identity();
    let ctm = Matrix::identity();
    font_m.scale(size as f64, size as f64);
    let opts = FontOptions::new();
    // context.get_font_face()
    let font_face = FontFace::toy_create(
        family,
        slant,
        weight
    );
    ScaledFont::new(&font_face, &font_m, &ctm, &opts)
}

/// Draw a text with horizontal and vertical extents centered
/// at the given cooridnate.
/// The last two arguments are a proportion of the text
/// extent that shuold be used to re-position the text
/// relative to the given coordinate.
pub fn draw_label(
    sf : &ScaledFont,
    ctx : &Context,
    label : &str,
    mut pos : Coord2D,
    rotate : bool,
    center : (bool, bool),
    off_x : Option<f64>,
    off_y : Option<f64>
) {
    ctx.save();
    let ext = sf.text_extents(label);
    let xadv = ext.x_advance;
    let height = ext.height;
    let half_xadv = xadv / 2.0;
    let half_height = height / 2.0;
    let x_center_off = match center.0 {
        true => (-1.0)*half_xadv,
        false => 0.0
    };
    let y_center_off = match center.1 {
        true => half_height,
        false => 0.0
    };
    let ext_off_x = off_x.unwrap_or(0.0)*xadv + x_center_off;
    let ext_off_y = off_y.unwrap_or(0.0)*height + y_center_off;
    pos.x += ext_off_x;
    pos.y += ext_off_y;
    let (glyphs, _) = sf.text_to_glyphs(pos.x, pos.y, label);
    let radius = (pos.x.powf(2.0) + pos.y.powf(2.0)).sqrt();
    if rotate {
        ctx.translate(-radius + height, radius);
        ctx.rotate(-PI/2.0);
    }
    ctx.show_glyphs(&glyphs[..]);
    ctx.restore();
}
