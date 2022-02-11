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
        crate::migrator::{
            Migrator,
            migrateto::MigrateTo,
        },
        worm::core::DbCtx,
    };
    pub trait Migrations {
        fn get_mig_path() -> &'static str;
    }
    pub trait DoMigrations: Migrations {
        fn migrate_up<'m, U>(migrate_to: Option<&'m str>) -> usize
        where
            U: DbCtx;
        fn migrate_down<'m, U>(migrate_to: Option<&'m str>) -> usize
        where
            U: DbCtx;
    }
    impl<T> DoMigrations for T
    where
        T: Migrations,
    {
        fn migrate_up<'m, U>(migrate_to: Option<&'m str>) -> usize
        where
            U: DbCtx
        {
            let mig_to = match migrate_to {
                Some(mig_name) => MigrateTo::StopAt(mig_name.to_string()),
                None => MigrateTo::NoStop,
            };
            let skips = match Migrator::<U>::do_up(Self::get_mig_path(), mig_to) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
        fn migrate_down<'m, U>(migrate_to: Option<&'m str>) -> usize
        where
            U: DbCtx
        {
            let mig_to = match migrate_to {
                Some(mig_name) => MigrateTo::StopAt(mig_name.to_string()),
                None => MigrateTo::NoStop,
            };
            let skips = match Migrator::<U>::do_down(Self::get_mig_path(), mig_to) {
                Ok(res) => res,
                Err(e) => panic!("{}", e),
            };
            return skips;
        }
    }
}

