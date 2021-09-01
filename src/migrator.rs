use super::{
    Connection,
    copy,
    Path,
    Ordering,
};
mod migrationdirection;
use migrationdirection::MigrationDirection;
mod domigrations;
use domigrations::DoMigrations;
mod testmigrator;
use testmigrator::TestMigrator;
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
    /// A test version of the Migrator
    test_self: Option<TestMigrator>,
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
    pub fn run_from_memory<'a>(migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::Memory) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        match m.upward_migration(migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
        return m.downward_migration(migrations_path);
    }
    pub fn do_up<'a>(db_path: &'a str, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let test_db_path = format!("{}.safe.db", db_path);
        let mut m = match Migrator::init(ConnectionType::SafeDbFile(db_path, &test_db_path)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        match std::fs::remove_file(test_db_path) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to remove file: {}", e)),
        }
        return m.upward_migration(migrations_path);
    }
    pub fn do_up_no_test<'a>(db_path: &'a str, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path);
    }
    pub fn do_down<'a>(db_path: &'a str, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let test_db_path = format!("{}.safe.db", db_path);
        let mut m = match Migrator::init(ConnectionType::SafeDbFile(db_path, &test_db_path)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        match std::fs::remove_file(test_db_path) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to remove file: {}", e)),
        }
        return m.downward_migration(migrations_path);
    }
    pub fn do_down_no_test<'a>(db_path: &'a str, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(ConnectionType::DbFile(db_path)) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path);
    }
    /// Initializes a new Migrator object
    fn init<'b>(c_type: ConnectionType) -> Result<Migrator, String> {
        let c = match Self::create_connection(c_type.clone()) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };
        match c_type {
            ConnectionType::Memory => {
                return Ok(Migrator { connection: c, skip_count: 0, test_self: None, });
            },
            ConnectionType::DbFile(_) => {
                return Ok(Migrator { connection: c, skip_count: 0, test_self: None, });
            },
            ConnectionType::SafeDbFile(c_str, tc_str) => {
                let c_path = Path::new(c_str);
                let path = Path::new(tc_str);
                if c_path.cmp(&path).eq(&Ordering::Equal) {
                    return Err(format!("Db file path {} and test db file path {} reference the same file system object", c_str, tc_str));
                } else if path.exists() {
                    return Err(format!("File system object {} already exists", tc_str));
                }
                match copy(c_path, path) {
                    Ok(_) => {},
                    Err(e) => return Err(format!("Failed to copy file system object {} to {}: {}", c_str, tc_str, e)),
                }
                let tm = match TestMigrator::init(ConnectionType::DbFile(tc_str)) {
                    Ok(tm) => tm,
                    Err(e) => return Err(format!("Failed to open test db connection to {}: {}", tc_str, e)),
                };
                return Ok(Migrator { connection: c, skip_count: 0, test_self: Some(tm), });
            },
        }
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        if self.test_self.is_some() {
            let test = self.test_self.as_mut().unwrap();
            match test.up(mig_path) {
                Ok(_) => {},
                Err(e) => {
                    return Err(format!("Test migration returned an error: {}", e));
                },
            }
        }
        return self.up(mig_path);
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        if self.test_self.is_some() {
            let test = self.test_self.as_mut().unwrap();
            match test.down(mig_path) {
                Ok(_) => {},
                Err(e) => {
                    return Err(format!("Test migration returned an error: {}", e));
                },
            }
        }
        return self.down(mig_path);
    }
    #[cfg(test)]
    fn do_both<'a>(db_path: &'a str, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let test_db_path = format!("{}.safe.db", db_path);
        let mut m = match Migrator::init(ConnectionType::SafeDbFile(db_path, &test_db_path)) {
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
        match std::fs::remove_file(test_db_path) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to remove file: {}", e)),
        }
        return Ok(up_skips + down_skips);
    }
}
#[cfg(test)]
mod migrator_tests {
    use crate::Migrator;
    #[test]
    fn t_file_up_down() {
        use std::fs::remove_file;
        const DB_LOC: &'static str = "./test.db";
        let skips = match Migrator::do_both(DB_LOC, "./test_sql") {
            Ok(skips) => skips,
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        };
        match remove_file(DB_LOC) {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to remove file {}: {}", DB_LOC, e);
                assert!(false);
            },
        }
        assert!(skips.eq(&0));
    }
}

