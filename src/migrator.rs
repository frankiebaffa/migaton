use super::Connection;
mod migrationdirection;
use migrationdirection::MigrationDirection;
mod domigrations;
use domigrations::DoMigrations;
mod migration;
use migration::Migration;
mod migratoraccess;
use migratoraccess::MigratorAccess;
/// A higher-level object to manage migration statuses
pub struct Migrator<'m> {
    /// A connection to a SQLite database
    connection: &'m mut Connection,
    /// A count of skipped migrations
    skip_count: usize,
}
impl<'m> MigratorAccess for Migrator<'m> {
    fn inc_skip_count(&mut self) {
        self.skip_count = self.skip_count + 1;
    }
    fn access_skip_count(&mut self) -> &mut usize {
        return &mut self.skip_count;
    }
    fn access_connection(&mut self) -> &mut Connection {
        return &mut self.connection;
    }
}
impl<'m> Migrator<'m> {
    /// Retrieves the number of migrations skipped due to unpassed checks (not innately an issue)
    pub fn get_skip_count(&self) -> usize {
        return self.skip_count.clone();
    }
    /// Retrieves whether or not the passed number matches the skip count
    pub fn chk_skip_count(&self, chk: usize) -> bool {
        return self.skip_count.eq(&chk);
    }
    /// Migrate upwards and downwards on a database stored in memory (used for testing)
    pub fn run_from_memory<'a>(c: &'m mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        match m.upward_migration(migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
        return m.downward_migration(migrations_path);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(c: &'m mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(c, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path);
    }
    /// Attempts to migrate upwards with no testing (not recommended)
    pub fn do_up_no_test<'a>(c: &'m mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(c: &'m mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(c, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path);
    }
    /// Attempts to migrate downwards with no testing (not recommended)
    pub fn do_down_no_test<'a>(c: &'m mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path);
    }
    fn init<'b>(c: &'m mut Connection) -> Result<Migrator<'m>, String> {
        return Ok(Migrator { connection: c, skip_count: 0, });
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.up(mig_path);
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.down(mig_path);
    }
    #[cfg(test)]
    fn do_both<'a>(m: &'m mut Connection, c: &'m mut Connection, migrations_path: &'a str, full_db_path: &'a str) -> Result<usize, String> {
        use std::fs::remove_file;
        match Migrator::run_from_memory(m, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        let up_skips = match m.upward_migration(migrations_path) {
            Ok(up_skips) => up_skips,
            Err(e) => return Err(e),
        };
        let down_skips = match m.downward_migration(migrations_path) {
            Ok(down_skips) => down_skips,
            Err(e) => return Err(e),
        };
        match remove_file(full_db_path) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
        return Ok(up_skips + down_skips);
    }
}
#[cfg(test)]
mod migrator_tests {
    use crate::Migrator;
    use rusqlite::Connection;
    #[test]
    fn t_file_up_down() {
        const FULL_DB_PATH: &'static str = "./Test.db";
        let mut m = match Connection::open(":memory:") {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };
        match m.execute("attach ':memory:' as Test", []) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
        let mut c = match Connection::open(FULL_DB_PATH) {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };
        match c.execute("attach 'Test.db' as Test", []) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
        let skips = match Migrator::do_both(&mut m, &mut c, "./test_sql", FULL_DB_PATH) {
            Ok(skips) => skips,
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        };
        assert!(skips.eq(&0));
    }
}

