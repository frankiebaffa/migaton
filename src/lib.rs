use sqlite::{
    Connection,
    State,
};
use std::io::Read;
use std::path::PathBuf;
use std::fs::File;
#[derive(PartialEq)]
pub enum MigrationDirection {
    Up,
    Down,
}
struct Migration {
    content: String,
}
impl Migration {
    const UP_END: &'static str = "up.sql";
    const DOWN_END: &'static str = "down.sql";
    fn get_migration<'a>(f_name: String, mig_path: &'a str) -> Result<Migration, String> {
        if !f_name.ends_with(Self::UP_END) && !f_name.ends_with(Self::DOWN_END) {
            return Err(format!("Invalid end of file name \"{}\"", f_name));
        }
        let mut f = match File::open(format!("{}/{}", mig_path, f_name)) {
            Ok(f) => f,
            Err(e) => return Err(format!("{}", e)),
        };
        let mut c = String::new();
        match f.read_to_string(&mut c) {
            Ok(_) => {},
            Err(e) => return Err(format!("{}", e)),
        };
        return Ok(Migration {
            content: c,
        });
    }
    pub fn get_all(direction: MigrationDirection, mig_path: String) -> Result<Vec<Migration>, String> {
        let p = PathBuf::from(mig_path.clone());
        if !p.is_dir() {
            return Err(format!("Migration directory {} does not exist", mig_path));
        }
        let dir = match p.read_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(format!("{}", e)),
        };
        let mut f_names: Vec<String> = Vec::new();
        for item_res in dir {
            let item = match item_res {
                Ok(item) => item,
                Err(e) => return Err(format!("{}", e)),
            };
            let t = match item.file_type() {
                Ok(t) => t,
                Err(e) => return Err(format!("{}", e)),
            };
            if !t.is_file() {
                continue;
            }
            let f_name_os = item.file_name().clone().to_owned();
            let f_name_opt = f_name_os.to_str();
            if f_name_opt.is_none() {
                return Err("Could not correctly retrieve file name from file".to_string());
            }
            let f_name = f_name_opt.as_ref().map(|s| s.to_string()).unwrap();
            if (f_name.ends_with(Self::UP_END) && direction.eq(&MigrationDirection::Up)) || (f_name.ends_with(Self::DOWN_END) && direction.eq(&MigrationDirection::Down)) {
                f_names.push(f_name);
            } else {
                continue;
            }
        }
        f_names.sort_unstable();
        if direction.eq(&MigrationDirection::Down) {
            f_names.reverse();
        }
        let mut migs = Vec::new();
        for mig in f_names {
            match Self::get_migration(mig, &mig_path) {
                Ok(m) => migs.push(m),
                Err(e) => return Err(format!("{}", e)),
            }
        }
        return Ok(migs);
    }
}
pub enum ConnectionType<'a> {
    Memory,
    DbFile(&'a str),
}
pub struct Migrator {}
impl Migrator {
    pub fn chk_dir() -> Result<(), String> {
        Ok(())
    }
    fn get_connection<'a>(c_type: ConnectionType) -> Result<Connection, String> {
        println!("getting connection");
        let c_str: &str = match c_type {
            ConnectionType::Memory => ":memory:",
            ConnectionType::DbFile(c_str) => c_str,
        };
        match Connection::open(c_str) {
            Ok(c) => return Ok(c),
            Err(e) => return Err(format!("{}", e)),
        };
    }
    fn migrate(c_type: ConnectionType, direction: MigrationDirection, mig_path: String) -> Result<(), String> {
        println!("beginning migration");
        let connection = match Self::get_connection(c_type) {
            Ok(connection) => connection,
            Err(e) => return Err(e),
        };
        let migrations = match Migration::get_all(direction, mig_path) {
            Ok(migrations) => migrations,
            Err(e) => return Err(e),
        };
        for migration in migrations {
            let mut stmt = match connection.prepare(migration.content) {
                Ok(stmt) => stmt,
                Err(e) => return Err(format!("{}", e)),
            };
            match stmt.next() {
                Ok(state) => match state {
                    State::Row => continue,
                    State::Done => continue,
                },
                Err(e) => return Err(format!("{}", e)),
            }
        }
        return Ok(());
    }
    pub fn up<'a>(c_type: ConnectionType, mig_path: &'a str) -> Result<(), String> {
        println!("migrating up");
        return Self::migrate(c_type, MigrationDirection::Up, mig_path.to_string());
    }
    pub fn down<'a>(c_type: ConnectionType, mig_path: &'a str) -> Result<(), String> {
        println!("migrating down");
        return Self::migrate(c_type, MigrationDirection::Down, mig_path.to_string());
    }
}
#[cfg(test)]
mod migrator_tests {
    use crate::{
        ConnectionType,
        Migrator,
    };
    #[test]
    fn t_memory_connect() {
        let ct = ConnectionType::Memory;
        let c = Migrator::get_connection(ct);
        assert!(c.is_ok());
    }
    #[test]
    fn t_memory_up() {
        let status = Migrator::up(ConnectionType::Memory, "./test_sql");
        if !status.is_ok() {
            match status.clone() {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
        }
        assert!(status.is_ok());
    }
    #[test]
    fn t_memory_down() {
        let status = Migrator::down(ConnectionType::Memory, "./test_sql");
        if !status.is_ok() {
            match status.clone() {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
        }
        assert!(status.is_ok());
    }
    //#[test]
    //fn t_file_connect() {
    //    let ct = ConnectionType::DbFile("./test.db");
    //    let c = Migrator::get_connection(ct);
    //    assert!(c.is_ok());
    //}
    //#[test]
    //fn t_file_up() {
    //    let status = Migrator::up(ConnectionType::DbFile("./test.db"), "./test_sql");
    //    if !status.is_ok() {
    //        match status.clone() {
    //            Ok(_) => {},
    //            Err(e) => println!("{}", e),
    //        }
    //    }
    //    assert!(status.is_ok());
    //}
    //#[test]
    //fn t_file_down() {
    //    let status = Migrator::down(ConnectionType::DbFile("./test.db"), "./test_sql");
    //    if !status.is_ok() {
    //        match status.clone() {
    //            Ok(_) => {},
    //            Err(e) => println!("{}", e),
    //        }
    //    }
    //    assert!(status.is_ok());
    //}
}

