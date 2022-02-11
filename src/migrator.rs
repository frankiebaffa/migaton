pub mod migrateto;
mod migrationdirection;
mod migration;
use {
    migrationdirection::MigrationDirection,
    migrateto::MigrateTo,
    migration::Migration,
    serde_yaml::Error as SerdeError,
    std::io::Error as IOError,
    worm::core::{
        sql::{
            Error as RusqliteError,
            Transaction,
        },
        DbCtx,
    },
};
#[derive(Debug)]
pub enum MigratorError {
    IOError(IOError),
    SerdeError(SerdeError),
    SQLError(RusqliteError),
    Error(String),
}
pub trait QuickMatch<T, U: std::error::Error>: Sized {
    fn quick_match(self) -> Result<T, MigratorError>;
}
impl std::fmt::Display for MigratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MigratorError::SQLError(e) => {
                let msg = &format!("{}", e);
                f.write_str(msg)
            },
            MigratorError::Error(s) => {
                f.write_str(&s)
            },
            MigratorError::IOError(e) => {
                let msg = &format!("{}", e);
                f.write_str(msg)
            },
            MigratorError::SerdeError(e) => {
                let msg = &format!("{}", e);
                f.write_str(msg)
            },
        }
    }
}
impl<T> QuickMatch<T, RusqliteError> for Result<T, RusqliteError> {
    fn quick_match(self) -> Result<T, MigratorError> {
        return match self {
            Ok(t) => Ok(t),
            Err(e) => Err(MigratorError::SQLError(e)),
        };
    }
}
impl<T> QuickMatch<T, IOError> for Result<T, IOError> {
    fn quick_match(self) -> Result<T, MigratorError> {
        return match self {
            Ok(t) => Ok(t),
            Err(e) => Err(MigratorError::IOError(e)),
        };
    }
}
impl<T> QuickMatch<T, SerdeError> for Result<T, SerdeError> {
    fn quick_match(self) -> Result<T, MigratorError> {
        return match self {
            Ok(t) => Ok(t),
            Err(e) => Err(MigratorError::SerdeError(e)),
        };
    }
}
/// A higher-level object to manage migration statuses
pub struct Migrator<T>
where
    T: DbCtx
{
    /// The worm database context
    db: T,
    /// The in-memory worm database context
    mem_db: T,
    /// A count of skipped migrations
    skip_count: usize,
}
impl<T> Migrator<T>
where
    T: DbCtx,
{
    fn inc_skip_count(&mut self) {
        self.skip_count = self.skip_count + 1;
    }
    /// Retrieves the number of migrations skipped due to unpassed checks (not innately an issue)
    pub fn get_skip_count(&self) -> usize {
        return self.skip_count.clone();
    }
    /// Retrieves whether or not the passed number matches the skip count
    pub fn chk_skip_count(&self, chk: usize) -> bool {
        return self.skip_count.eq(&chk);
    }
    /// Migrate upwards and downwards on a database stored in memory (used for testing)
    pub fn run_from_memory<'a>(migrations_path: &'a str) -> Result<usize, MigratorError> {
        let mut m = Migrator::<T>::init()?;
        let mut skips = m.upward_migration(migrations_path, true, MigrateTo::NoStop)?;
        skips = skips + m.downward_migration(migrations_path, true, MigrateTo::NoStop)?;
        if skips > 0 {
            return Err(MigratorError::Error(format!("Memory skips should always be 0, returned {}", skips)));
        }
        return Ok(skips);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(
        migrations_path: &'a str, migrate_to: MigrateTo,
    ) -> Result<usize, MigratorError> {
        Migrator::<T>::run_from_memory(migrations_path)?;
        let mut m = Migrator::<T>::init()?;
        return m.upward_migration(migrations_path, false, migrate_to);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(
        migrations_path: &'a str, migrate_to: MigrateTo
    ) -> Result<usize, MigratorError> {
        Migrator::<T>::run_from_memory(migrations_path)?;
        let mut m = Migrator::<T>::init()?;
        return m.downward_migration(migrations_path, false, migrate_to);
    }
    fn init() -> Result<Migrator<T>, MigratorError> {
        let mut mem_db = T::init();
        mem_db.attach_temp_dbs();
        let mut db = T::init();
        db.attach_dbs();
        return Ok(Migrator { db, mem_db, skip_count: 0, });
    }
    fn upward_migration<'b>(
        &mut self, mig_path: &'b str, is_test: bool, migrate_to: MigrateTo,
    ) -> Result<usize, MigratorError> {
        self.migrate(
            MigrationDirection::Up(migrate_to),
            mig_path.to_string(),
            is_test
        )
    }
    fn downward_migration<'b>(
        &mut self, mig_path: &'b str, is_test: bool, migrate_to: MigrateTo
    ) -> Result<usize, MigratorError> {
        return self.migrate(
            MigrationDirection::Down(migrate_to),
            mig_path.to_string(),
            is_test
        );
    }
    /// Runs the passed Migration's check script
    fn query_chk(&mut self, is_test: bool, m: &Migration) -> Result<i64, MigratorError> {
        let c = if is_test {
            self.mem_db.use_connection()
        } else {
            self.db.use_connection()
        };
        let mut chk_stmt = c.prepare(&m.check).quick_match()?;
        // TODO: change this back to a quick match.
        // returning 0 on error due to a bug in another program, checking how this works
        return match chk_stmt.query_row([], |row| row.get(0)) {
            Ok(i) => Ok(i),
            Err(_) => Ok(0),
        };
    }
    /// Runs the applicable script of a Migration based on the direction
    fn run_migration(t: &mut Transaction, m: &Migration, d: &MigrationDirection) -> Result<(), MigratorError> {
        let s = match d {
            &MigrationDirection::Up(_) => {
                m.up.clone()
            },
            &MigrationDirection::Down(_) => {
                m.down.clone()
            },
        };
        return t.execute_batch(&s).quick_match();
    }
    /// Migrates the SQLite database in the given direction
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String, is_test: bool) -> Result<usize, MigratorError> {
        const DOWN_DIR: &'static str = "down";
        const UP_DIR: &'static str = "up";
        let run_verb = if is_test {
            "Testing"
        } else {
            "Running"
        };
        let stop_name;
        let dir_str;
        match direction.clone() {
            MigrationDirection::Up(mig_to) => {
                match mig_to {
                    MigrateTo::StopAt(name) => {
                        stop_name = Some(name);
                    },
                    MigrateTo::NoStop => {
                        stop_name = None;
                    },
                }
                dir_str = UP_DIR;
            },
            MigrationDirection::Down(mig_to) => {
                match mig_to {
                    MigrateTo::StopAt(name) => {
                        stop_name = Some(name);
                    },
                    MigrateTo::NoStop => {
                        stop_name = None;
                    },
                }
                dir_str = DOWN_DIR;
            },
        };
        let mut migrations = Migration::get_all(mig_path)?;
        let passing_int: i64;
        if dir_str.eq(DOWN_DIR) {
            migrations.sort_unstable_by(|x, y| y.number.cmp(&x.number));
            passing_int = 1;
        } else {
            migrations.sort_unstable_by(|x, y| x.number.cmp(&y.number));
            passing_int = 0;
        }
        for migration in migrations {
            let passing_check = self.query_chk(is_test, &migration)?;
            let do_run = if dir_str.eq(DOWN_DIR) && passing_check.ge(&passing_int) {
                true
            } else if dir_str.eq(UP_DIR) && passing_check.eq(&passing_int) {
                true
            } else {
                false
            };
            if !do_run {
                self.inc_skip_count();
                println!("Skipping {} {}. Result {}", dir_str, migration.file_name, passing_check);
                continue;
            }
            println!("{} {} '{}'", run_verb, dir_str, migration.file_name);
            let con = if is_test {
                self.mem_db.use_connection()
            } else {
                self.db.use_connection()
            };
            let mut tx = con.transaction().quick_match()?;
            Self::run_migration(&mut tx, &migration, &direction)?;
            tx.commit().quick_match()?;
            if stop_name.is_some() && stop_name.clone().unwrap().eq(&migration.file_name) {
                break;
            }
        }
        return Ok(self.get_skip_count());
    }
}
#[cfg(test)]
mod migrator_tests {
    use {
        crate::traits::{ Migrations, DoMigrations },
        worm::core::{DbCtx, DbContext},
        worm::derive::WormDb,
    };
    #[derive(WormDb)]
    #[db(var(name="WORMDBS"))]
    struct TestDb {
        context: DbContext,
    }
    struct TestMigrator;
    impl TestMigrator {
        const MIG_PATH: &'static str = "./test_sql";
    }
    impl Migrations for TestMigrator {
        fn get_mig_path() -> &'static str {
            Self::MIG_PATH
        }
    }
    #[test]
    fn t_file_up_down() {
        use std::fs::remove_file;
        const DB_PATH: &'static str = "./";
        const DB_NAME: &'static str = "Test";
        let mut mem_testdb = TestDb::init();
        mem_testdb.context.attach_temp_dbs();
        let mut testdb = TestDb::init();
        testdb.context.attach_dbs();
        let up_skips = TestMigrator::migrate_up::<TestDb>(None);
        let down_skips = TestMigrator::migrate_down::<TestDb>(None);
        match remove_file(&format!("{}/{}.db", DB_PATH, DB_NAME)) {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
            },
        }
        assert!((up_skips + down_skips).eq(&0));
    }
}

