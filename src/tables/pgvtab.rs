use rusqlite::vtab::*;
use std::default::Default;

#[repr(C)]
struct PGVTab {

   base: ffi::sqlite3_vtab,
   /* Virtual table implementations will typically add additional fields */
}

unsafe impl<'vtab> VTab<'vtab> for PGVTab {

    // (connection, table)
    type Aux = (String, String);

    type Cursor: VTabCursor<'vtab>;

    fn connect(
        db: &mut VTabConnection,
        aux: Option<&Self::Aux>,
        args: &[&[u8]]
    ) -> Result<(String, Self)> {
        let vtab = PGVTab {
            base: sqlite3_vtab::default(),
        };
        Ok(("CREATE TABLE x(value)".to_owned(), vtab))
    }

    fn best_index(&self, info: &mut IndexInfo) -> Result<()> {
        Ok(())
    }

    fn open(&'vtab self) -> Result<Self::Cursor> {
         Ok(DummyTabCursor::default())
    }
}

#[derive(Default)]
#[repr(C)]
struct PGVTabCursor<'vtab> {
    row_id: i64,
    &'vtab PGVTab
}

unsafe impl VTabCursor for DummyTabCursor<'_> {

    fn filter(
        &mut self,
        _idx_num: c_int,
        _idx_str: Option<&str>,
        _args: &Values<'_>,
    ) -> Result<()> {
        self.row_id = 1;
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        self.row_id += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        self.row_id > 1
    }

    fn column(&self, ctx: &mut Context, _: c_int) -> Result<()> {
        ctx.set_result(&self.row_id)
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.row_id)
    }
}

// planned usage create virtual table patients('https::/pgconn', 'select * from my_tbl');
// Then queries to patients will be forwarded to the query informed at the vtab creation.
pub fn register_pgvtab(conn : &Connection, conn_str : &str, tbl : &str) {
    let aux = (conn_str.to_string(), tbl.to_string());
    let module = eponymous_only_module::<PGVTab>();
    if let Err(e) = conn.create_module("pgvtab", &module, Some(aux)) {
        println!("{}", e);
    }
    // let mut s = db.prepare("SELECT * FROM dummy()").unwrap();
    // let dummy = s
    //    .query_row(&[] as &[&dyn ToSql], |row| row.get::<_, i32>(0))
    //    .unwrap();
    assert_eq!(1, dummy);
}
