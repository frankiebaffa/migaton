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
    pub trait DoMigrations: Migrations {
        fn migrate_up<U>() -> usize
        where
            U: DbCtx;
        fn migrate_down<U>() -> usize
        where
            U: DbCtx;
    }
    impl<T> DoMigrations for T
    where
        T: Migrations,
    {
        fn migrate_up<U>() -> usize
        where
            U: DbCtx
        {
            let skips = match Migrator::<U>::do_up(Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
        fn migrate_down<U>() -> usize
        where
            U: DbCtx
        {
            let skips = match Migrator::<U>::do_down(Self::get_mig_path()) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
    }
}

