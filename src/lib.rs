use std::{
    fs::File,
    path::{
        Path,
        PathBuf,
    },
};
mod migrator;
pub use migrator::Migrator;
pub mod traits {
    use {
        crate::migrator::Migrator,
        worm::core::DbCtx,
    };
    pub trait Migrations {
        fn get_mig_path() -> &'static str;
    }
    pub trait DoMigrations<U>: Migrations
    where
        U: DbCtx
    {
        fn migrate_up() -> usize;
        fn migrate_down() -> usize;
    }
    impl<T, U> DoMigrations<U> for T
    where
        T: Migrations,
        U: DbCtx,
    {
        fn migrate_up() -> usize {
            let skips = match Migrator::<U>::do_up(Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
        fn migrate_down() -> usize {
            let skips = match Migrator::<U>::do_down(Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
    }
}

