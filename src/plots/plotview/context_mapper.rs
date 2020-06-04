// use cairo::Context;
// use std::collections::HashMap;
// use std::f64::consts::PI;
use std::ops::Add;

#[derive(Clone, Copy, Debug)]
pub struct Coord2D {
    pub x : f64,
    pub y : f64
}

impl Coord2D {
    pub fn new(x : f64, y : f64) -> Coord2D {
        Coord2D{x, y}
    }

    pub fn distance(&self, other : Coord2D) -> f64 {
        ((self.x - other.x).powf(2.0) +
            (self.y - other.y).powf(2.0)).sqrt()
    }
}

impl Add for Coord2D {

    type Output=Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Default)]
pub struct ContextMapper {
    pub xmin : f64,
    pub xmax : f64,
    pub ymin : f64,
    pub ymax : f64,
    pub xlog : bool,
    pub ylog : bool,
    pub xinv : bool,
    pub yinv : bool,
    pub xext : f64,
    pub yext : f64,
    pub w : i32,
    pub h : i32,
}

impl ContextMapper {

    //fn new() -> ContextMapper {
    //    ContextMapper{..Default::default()}
    //}

    pub fn new(
        xmin : f64, xmax : f64, ymin : f64, ymax : f64,
        xlog : bool, ylog : bool, xinv : bool, yinv : bool)
        -> ContextMapper {

        let (w, h) = (0, 0);
        let (xext, yext) = ContextMapper::calc_ext(
            xmax, xmin, ymax, ymin, xlog, ylog);
        ContextMapper{ xmin, xmax, ymin, ymax,
        xext, yext, w, h, xlog, ylog, xinv, yinv}
    }

    pub fn update_data_extensions(&mut self, xmin : f64, xmax : f64, ymin : f64, ymax : f64) {
        self.xmin = xmin;
        self.xmax = xmax;
        self.ymin = ymin;
        self.ymax = ymax;
    }

    pub fn update_dimensions(&mut self, w : i32, h : i32) {
        self.w = w;
        self.h = h;
    }

    pub fn calc_ext(xmax : f64, xmin : f64, ymax : f64, ymin : f64,
        xlog : bool, ylog : bool) -> (f64, f64) {
        let xext = match xlog {
            true => (xmax.log10() - xmin.log10()).abs(),
            false => (xmax - xmin).abs()
        };
        let yext = match ylog {
            true => (ymax.log10() - ymin.log10()).abs(),
            false => (ymax - ymin).abs()
        };
        (xext, yext)
    }

    pub fn set_mode(
        &mut self, xinv : bool, xlog : bool, yinv : bool, ylog : bool) {
        self.xlog = xlog;
        self.xinv = xinv;
        self.ylog = ylog;
        self.yinv = yinv;
        let (xext, yext) = ContextMapper::calc_ext(
            self.xmax, self.xmin, self.ymax, self.ymin, xlog, ylog);
        self.xext = xext;
        self.yext = yext;
    }

    pub fn map(&self, x : f64, y : f64) -> Coord2D {
        let padw = 0.1*(self.w as f64);
        let padh = 0.1*(self.h as f64);
        let dataw = (self.w as f64) - 2.0*padw;
        let datah = (self.h as f64) - 2.0*padh;

        let xprop = match (self.xlog, self.xinv) {
            (false, false) => (x - self.xmin) / self.xext,
            (false, true)  => (self.xmax - x) / self.xext,
            (true, false)  => (x.log10() - self.xmin.log10()) / self.xext,
            (true, true)   => (self.xmin.log10() - x.log10()) / self.xext
        };
        let yprop = 1.0 - match (self.ylog, self.yinv) { // Here
            (false, false) => (y - self.ymin) / self.yext,
            (false, true)  => (self.ymax - y) / self.yext,
            (true, false)  => (y.log10() - self.ymin.log10()) / self.yext,
            (true, true)   => (self.ymin.log10() - y.log10()) / self.yext
        };

        //println!("{:?} {:?} {:?} {:?}", self.xlog, self.xinv, self.ylog,self.yinv);

        Coord2D::new(padw + dataw*xprop, padh + datah*yprop)
    }

    pub fn check_bounds(&self, x : f64, y : f64) -> bool {
        let x_ok = x >= self.xmin && x <= self.xmax;
        let y_ok = y >= self.ymin && y <= self.ymax;
        /*match self.xinv {
            false => x >= self.xmin && x <= self.xmax,
            true => x <= self.xmin && x >= self.xmax
        };*/
        /*let y_ok = match self.yinv {
            false => y >= self.ymin && y <= self.ymax,
            true => y <= self.ymin && y >= self.ymax
        };*/
        x_ok && y_ok
    }

    pub fn coord_bounds(&self) -> (Coord2D, Coord2D, Coord2D, Coord2D) {
        (
            self.map(self.xmin, self.ymin),
            self.map(self.xmax, self.ymin),
            self.map(self.xmax, self.ymax),
            self.map(self.xmin, self.ymax)
        )
    }

    pub fn data_extensions(&self) -> (f64, f64, f64, f64) {
        (self.xmin, self.xmax, self.ymin, self.ymax)
    }

    pub fn coord_extensions(&self) -> (f64, f64) {
        let x_ext = self.map(self.xmin, self.ymin)
            .distance(self.map(self.xmax, self.ymin));
        let y_ext = self.map(self.xmin, self.ymin)
            .distance(self.map(self.xmin, self.ymax));
        (x_ext, y_ext)
    }

}

