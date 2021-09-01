use rusqlite::{
    Connection,
    Transaction
};
use std::io::Read;
use std::path::{
    Path,
    PathBuf
};
use std::fs::File;
#[derive(PartialEq)]
pub enum MigrationDirection {
    Up,
    Down,
}
struct Migaton {
    script: String,
}
impl Migaton {
    fn new(script: String) -> Migaton {
        return Migaton { script, };
    }
}
struct Migration {
    number: i64,
    up: Migaton,
    down: Migaton,
    check: Migaton,
}
impl Migration {
    fn new<'a>(number: i64, up: Migaton, down: Migaton, check: Migaton) -> Migration {
        return Migration { number, up, down, check, };
    }
    const UP_END: &'static str = "up.sql";
    const DOWN_END: &'static str = "down.sql";
    const CHK_END: &'static str = "chk.sql";
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
                Err(_) => return Err(format!("Failed to open file {}", up_file_name)),
            };
            let mut up_script = String::new();
            match up_file.read_to_string(&mut up_script) {
                Ok(_) => {},
                Err(_) => return Err(format!("Failed to read {} to string", up_file_name)),
            };
            let up = Migaton::new(up_script);
            let mut down_file = match File::open(&down_file_name) {
                Ok(down_file) => down_file,
                Err(_) => return Err(format!("Failed to open file {}", down_file_name)),
            };
            let mut down_script = String::new();
            match down_file.read_to_string(&mut down_script) {
                Ok(_) => {},
                Err(_) => return Err(format!("Failed to read {} to string", down_file_name)),
            };
            let down = Migaton::new(down_script);
            let mut chk_file = match File::open(&chk_file_name) {
                Ok(chk_file) => chk_file,
                Err(_) => return Err(format!("Failed to open file {}", chk_file_name)),
            };
            let mut chk_script = String::new();
            match chk_file.read_to_string(&mut chk_script) {
                Ok(_) => {},
                Err(_) => return Err(format!("Failed to read {} to string", chk_file_name)),
            };
            let chk = Migaton::new(chk_script);
            migrations.push(Migration::new(index, up, down, chk));
            index = index + 1;
        }
    }
}
pub enum ConnectionType<'a> {
    Memory,
    DbFile(&'a str),
}
pub struct Migrator<'a> {
    connection: &'a mut Connection,
    skip_count: usize,
}
impl<'a> Migrator<'a> {
    pub fn get_skip_count(&self) -> usize {
        return self.skip_count.clone();
    }
    pub fn chk_dir() -> Result<(), String> {
        Ok(())
    }
    pub fn create_connection(c_type: ConnectionType) -> Result<Connection, String> {
        let c_str: &str = match c_type {
            ConnectionType::Memory => ":memory:",
            ConnectionType::DbFile(c_str) => c_str,
        };
        match Connection::open(c_str) {
            Ok(c) => return Ok(c),
            Err(_) => return Err(format!("Failed to open connection to {}", c_str)),
        };
    }
    pub fn init(connection: &mut Connection) -> Migrator {
        return Migrator { connection, skip_count: 0 };
    }
    fn query_chk(c: &Connection, script: String) -> Result<i64, String> {
        let mut chk_stmt = match c.prepare(&script) {
            Ok(stmt) => stmt,
            Err(_) => return Err("Failed to prepare statement".to_string()),
        };
        match chk_stmt.query_row([], |row| row.get(0)) {
            Ok(i) => return Ok(i),
            Err(_) => return Err("Failed to query rows from statement".to_string()),
        };
    }
    fn run_migration(t: &mut Transaction, script: String) -> Result<(), String> {
        match t.execute(&script, []) {
            Ok(_) => return Ok(()),
            Err(_) => return Err("Failed to execute script".to_string()),
        };
    }
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String) -> Result<(), String> {
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
            let passing_check = match Self::query_chk(&self.connection, migration.check.script) {
                Ok(i) => i,
                Err(e) => return Err(e),
            };
            if !passing_int.eq(&passing_check) {
                self.skip_count = self.skip_count + 1;
                continue;
            }
            let script = match direction {
                MigrationDirection::Up => {
                    migration.up.script
                },
                MigrationDirection::Down => {
                    migration.down.script
                },
            };
            let mut tx = match self.connection.transaction() {
                Ok(tx) => tx,
                Err(_) => return Err("Failed to create transaction from connection".to_string()),
            };
            match Self::run_migration(&mut tx, script) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
            match tx.commit() {
                Ok(_) => continue,
                Err(_) => return Err("Failed to commit transaction".to_string()),
            }
        }
        return Ok(());
    }
    pub fn up<'b>(&mut self, mig_path: &'b str) -> Result<(), String> {
        return self.migrate(MigrationDirection::Up, mig_path.to_string());
    }
    pub fn down<'b>(&mut self, mig_path: &'b str) -> Result<(), String> {
        return self.migrate(MigrationDirection::Down, mig_path.to_string());
    }
}
#[cfg(test)]
mod migrator_tests {
    use crate::{
        ConnectionType,
        Migrator,
    };
    #[test]
    fn t_memory_up_down() {
        let ct = ConnectionType::Memory;
        let c = Migrator::create_connection(ct);
        if !c.is_ok() {
            match c {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
            assert!(false);
            return;
        }
        let mut con = c.unwrap();
        let mut migrator = Migrator::init(&mut con);
        match migrator.up("./test_sql") {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        }
        match migrator.down("./test_sql") {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        }
        assert!(migrator.get_skip_count().eq(&0));
    }
    #[test]
    fn t_file_up_down() {
        const DB_LOC: &'static str = "./test.db";
        let ct = ConnectionType::DbFile(DB_LOC);
        let c = Migrator::create_connection(ct);
        if !c.is_ok() {
            match c {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
            assert!(false);
            return;
        }
        let mut con = c.unwrap();
        let mut migrator = Migrator::init(&mut con);
        match migrator.up("./test_sql") {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        }
        match migrator.down("./test_sql") {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
                return;
            },
        }
        use std::fs::remove_file;
        match remove_file(DB_LOC) {
            Ok(_) => {},
            Err(_) => {
                println!("Failed to remove file {}", DB_LOC);
                assert!(false);
            },
        }
        assert!(migrator.get_skip_count().eq(&0));
    }
}

