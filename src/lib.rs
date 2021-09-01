use rusqlite::{
    Connection,
    Transaction
};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{
    Path,
    PathBuf
};
use std::fs::{
    File,
    copy,
};
/// A representation of the direction of a migration
#[derive(PartialEq)]
pub enum MigrationDirection {
    /// An upward migration
    Up,
    /// A downward migration
    Down,
}
/// A database migration containing upward, downward, and check scripts
struct Migration {
    /// A number denoting the ordering of the migration
    number: i64,
    /// An upward migration script
    up: String,
    /// A downward migration script
    down: String,
    /// A check migration script
    check: String,
}
impl Migration {
    /// Creates a new migration
    fn new<'a>(number: i64, up: String, down: String, check: String) -> Migration {
        return Migration { number, up, down, check, };
    }
    const UP_END: &'static str = "up.sql";
    const DOWN_END: &'static str = "down.sql";
    const CHK_END: &'static str = "chk.sql";
    /// Retrieves all migrations from the given path
    pub fn get_all(mig_path: String) -> Result<Vec<Migration>, String> {
        let p = PathBuf::from(mig_path.clone());
        if !p.is_dir() {
            return Err(format!("Migration directory {} does not exist", mig_path));
        }
        let mut migrations: Vec<Migration> = Vec::new();
        let mut index = 1;
        loop {
            let up_file_name = format!("{}/{}.{}", mig_path, index, Self::UP_END);
            let up_exists = Path::new(&up_file_name).exists();
            let down_file_name = format!("{}/{}.{}", mig_path, index, Self::DOWN_END);
            let down_exists = Path::new(&down_file_name).exists();
            let chk_file_name = format!("{}/{}.{}", mig_path, index, Self::CHK_END);
            let chk_exists = Path::new(&chk_file_name).exists();
            if !up_exists && !down_exists && !chk_exists {
                return Ok(migrations);
            } else if !up_exists || !down_exists || !chk_exists {
                return Err(format!("Incomplete set for migration {}", index));
            }
            let mut up_file = match File::open(&up_file_name) {
                Ok(up_file) => up_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", up_file_name, e)),
            };
            let mut up_script = String::new();
            match up_file.read_to_string(&mut up_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", up_file_name, e)),
            };
            let mut down_file = match File::open(&down_file_name) {
                Ok(down_file) => down_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", down_file_name, e)),
            };
            let mut down_script = String::new();
            match down_file.read_to_string(&mut down_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", down_file_name, e)),
            };
            let mut chk_file = match File::open(&chk_file_name) {
                Ok(chk_file) => chk_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", chk_file_name, e)),
            };
            let mut chk_script = String::new();
            match chk_file.read_to_string(&mut chk_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", chk_file_name, e)),
            };
            migrations.push(Migration::new(index, up_script, down_script, chk_script));
            index = index + 1;
        }
    }
}
/// A representation of the type of SQLite connection being opened
#[derive(PartialEq, Clone)]
enum ConnectionType<'a> {
    /// A connection to a temporary, in-memory SQLite database
    Memory,
    /// A connection to a SQLite database file
    DbFile(&'a str),
    /// A connection to a SQLite database file alongside a copy of said db file
    SafeDbFile(&'a str, &'a str),
}
trait MigratorAccess {
    fn access_skip_count(&mut self) -> &mut usize;
    fn access_connection(&mut self) -> &mut Connection;
    fn inc_skip_count(&mut self);
}
/// A test version of Migrator
pub struct TestMigrator {
    /// A connection to a SQLite database
    connection: Connection,
    /// A count of skipped migrations
    skip_count: usize,
}
impl MigratorAccess for TestMigrator {
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
trait DoMigrations {
    fn get_skip_count(&mut self) -> usize;
    fn create_connection(c_type: ConnectionType) -> Result<Connection, String>;
    fn query_chk(c: &Connection, m: &Migration) -> Result<i64, String>;
    fn run_migration(t: &mut Transaction, m: &Migration, d: &MigrationDirection) -> Result<(), String>;
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String) -> Result<usize, String>;
    fn up<'b>(&mut self, mig_path: &'b str) -> Result<usize, String>;
    fn down<'b>(&mut self, mig_path: &'b str) -> Result<usize, String>;
}
impl<T> DoMigrations for T where T: MigratorAccess {
    /// Creates a connection to a SQLite database
    fn create_connection(c_type: ConnectionType) -> Result<Connection, String> {
        let c_str: &str = match c_type {
            ConnectionType::Memory => ":memory:",
            ConnectionType::DbFile(c_str) => c_str,
            ConnectionType::SafeDbFile(c_str, _) => c_str,
        };
        match Connection::open(c_str) {
            Ok(c) => return Ok(c),
            Err(e) => return Err(format!("Failed to open connection to {}: {}", c_str, e)),
        };
    }
    /// Retrieves the skip count
    fn get_skip_count(&mut self) -> usize {
        return self.access_skip_count().clone();
    }
    /// Runs the passed Migration's check script
    fn query_chk(c: &Connection, m: &Migration) -> Result<i64, String> {
        let mut chk_stmt = match c.prepare(&m.check) {
            Ok(stmt) => stmt,
            Err(e) => return Err(format!("Failed to prepare statement: {}", e)),
        };
        match chk_stmt.query_row([], |row| row.get(0)) {
            Ok(i) => return Ok(i),
            Err(e) => return Err(format!("Failed to query rows from statement: {}", e)),
        };
    }
    /// Runs the applicable script of a Migration based on the direction
    fn run_migration(t: &mut Transaction, m: &Migration, d: &MigrationDirection) -> Result<(), String> {
        let s = match d {
            &MigrationDirection::Up => {
                m.up.clone()
            },
            &MigrationDirection::Down => {
                m.down.clone()
            },
        };
        match t.execute(&s, []) {
            Ok(_) => return Ok(()),
            Err(e) => return Err(format!("Failed to execute script: {}", e)),
        };
    }
    /// Migrates the SQLite database in the given direction
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String) -> Result<usize, String> {
        let mut migrations = match Migration::get_all(mig_path) {
            Ok(migrations) => migrations,
            Err(e) => return Err(e),
        };
        let passing_int: i64;
        if direction.eq(&MigrationDirection::Down) {
            migrations.sort_unstable_by(|x, y| y.number.cmp(&x.number));
            passing_int = 1;
        } else {
            migrations.sort_unstable_by(|x, y| x.number.cmp(&y.number));
            passing_int = 0;
        }
        for migration in migrations {
            let passing_check = match Self::query_chk(&self.access_connection(), &migration) {
                Ok(i) => i,
                Err(e) => return Err(e),
            };
            if !passing_int.eq(&passing_check) {
                self.inc_skip_count();
                continue;
            }
            let mut tx = match self.access_connection().transaction() {
                Ok(tx) => tx,
                Err(e) => return Err(format!("Failed to create transaction from connection: {}", e)),
            };
            match Self::run_migration(&mut tx, &migration, &direction) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
            match tx.commit() {
                Ok(_) => continue,
                Err(e) => return Err(format!("Failed to commit transaction: {}", e)),
            }
        }
        return Ok(self.get_skip_count());
    }
    /// Migrates the SQLite database upward
    fn up<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.migrate(MigrationDirection::Up, mig_path.to_string());
    }
    /// Migrates the SQLite database downward
    fn down<'b>(&mut self, mig_path: &'b str) -> Result<usize, String> {
        return self.migrate(MigrationDirection::Down, mig_path.to_string());
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
impl TestMigrator {
    /// Initializes a new TestMigrator object
    fn init<'b>(c_type: ConnectionType) -> Result<TestMigrator, String> {
        let c = match Self::create_connection(c_type.clone()) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };
        match c_type {
            ConnectionType::Memory => {
                return Ok(TestMigrator { connection: c, skip_count: 0 });
            },
            ConnectionType::DbFile(_) => {
                return Ok(TestMigrator { connection: c, skip_count: 0, });
            },
            ConnectionType::SafeDbFile(_, _) => {
                return Err("Cannot create a test migrator within a test migrator".to_string());
            },
        }
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

