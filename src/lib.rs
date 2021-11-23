use rusqlite::{
    Connection,
    Transaction
};
use std::path::{
    Path,
    PathBuf
};
use std::fs::File;
mod migrator;
pub use migrator::Migrator;
pub mod traits {
    use worm::core::DbCtx;
    use crate::migrator::Migrator;
    pub trait Migrations {
        fn get_mig_path() -> &'static str;
    }
    pub trait DoMigrations: Migrations {
        fn migrate_up(mem_db: &mut impl DbCtx, db: &mut impl DbCtx) -> usize;
        fn migrate_down(mem_db: &mut impl DbCtx, db: &mut impl DbCtx) -> usize;
    }
    impl<T> DoMigrations for T where T: Migrations {
        fn migrate_up(mem_db: &mut impl DbCtx, db: &mut impl DbCtx) -> usize {
            let mut mem_c = mem_db.use_connection();
            let mut c = db.use_connection();
            let skips = match Migrator::do_up(&mut mem_c, &mut c, Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
        fn migrate_down(mem_db: &mut impl DbCtx, db: &mut impl DbCtx) -> usize {
            let mut mem_c = mem_db.use_connection();
            let mut c = db.use_connection();
            let skips = match Migrator::do_down(&mut mem_c, &mut c, Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
    }
}

