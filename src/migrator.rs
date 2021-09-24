use super::Connection;
mod migrationdirection;
use migrationdirection::MigrationDirection;
mod domigrations;
use domigrations::DoMigrations;
mod connectiontype;
use connectiontype::ConnectionType;
mod migration;
use migration::Migration;
mod migratoraccess;
use migratoraccess::MigratorAccess;
/// A higher-level object to manage migration statuses
pub struct Migrator {
    /// A connection to a SQLite database
    connection: Connection,
    /// A count of skipped migrations
    skip_count: usize,
}
impl MigratorAccess for Migrator {
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
impl Migrator {
    /// Retrieves the number of migrations skipped due to unpassed checks (not innately an issue)
    pub fn get_skip_count(&self) -> usize {
        return self.skip_count.clone();
    }
    /// Retrieves whether or not the passed number matches the skip count
    pub fn chk_skip_count(&self, chk: usize) -> bool {
        return self.skip_count.eq(&chk);
    }
    /// Migrate upwards and downwards on a database stored in memory (used for testing)
    pub fn run_from_memory<'a>(migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::Memory(db_name)) {
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
    pub fn do_up<'a>(db_path: &'a str, migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path, db_name)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path);
    }
    /// Attempts to migrate upwards with no testing (not recommended)
    pub fn do_up_no_test<'a>(db_path: &'a str, migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path, db_name)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(db_path: &'a str, migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path, db_name)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path);
    }
    /// Attempts to migrate downwards with no testing (not recommended)
    pub fn do_down_no_test<'a>(db_path: &'a str, migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path, db_name)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path);
    }
    fn init<'b>(c_type: ConnectionType) -> Result<Migrator, String> {
        let c = match Self::create_connection(c_type.clone()) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };
        match c_type {
            ConnectionType::Memory(_) => {
                return Ok(Migrator { connection: c, skip_count: 0, });
            },
            ConnectionType::DbFile(_, _) => {
                return Ok(Migrator { connection: c, skip_count: 0, });
            },
        }
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.up(mig_path);
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.down(mig_path);
    }
    #[cfg(test)]
    fn do_both<'a>(db_path: &'a str, migrations_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        use std::fs::remove_file;
        match Migrator::run_from_memory(migrations_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let test_c = ConnectionType::DbFile(db_path, db_name);
        let mut m = match Migrator::init(test_c.clone()) {
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
        match remove_file(test_c.get_full_db_path()) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
        return Ok(up_skips + down_skips);
    }
}
#[cfg(test)]
mod migrator_tests {
    use crate::Migrator;
    #[test]
    fn t_file_up_down() {
        const DB_LOC: &'static str = "./";
        const DB_NAME: &'static str = "Test";
        let skips = match Migrator::do_both(DB_LOC, "./test_sql", DB_NAME) {
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

