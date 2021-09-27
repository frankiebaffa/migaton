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
    /// The db path
    db_path: &'m str,
    /// The db name
    db_name: &'m str,
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
    fn attach(&mut self, is_mem: bool) -> Result<usize, rusqlite::Error> {
        let db_name = self.db_name.clone();
        let db_path = self.db_path.clone();
        let c = self.access_connection();
        let cmd = if is_mem {
            format!("attach ':memory:' as {}", db_name)
        } else {
            format!("attach '{}/{}.db' as {}", db_path, db_name, db_name)
        };
        return Ok(c.execute(&cmd, [])?);
    }
    fn detach(&mut self) -> Result<usize, rusqlite::Error> {
        let db_name = self.db_name.clone();
        let c = self.access_connection();
        return Ok(c.execute(&format!("detach {}", db_name), [])?);
    }
    /// Migrate upwards and downwards on a database stored in memory (used for testing)
    pub fn run_from_memory<'a>(c: &'m mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(c, db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        match m.attach(true) {
            Ok(_) => {},
            Err(e) => return Err(format!("{}", e)),
        }
        let mut skips = match m.upward_migration(migrations_path, true) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        skips = skips + match m.downward_migration(migrations_path, true) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        match m.detach() {
            Ok(_) => {},
            Err(e) => return Err(format!("{}", e)),
        }
        if skips > 0 {
            return Err(format!("Memory skips should always be 0, returned {}", skips));
        }
        return Ok(skips);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(c: &'m mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(c, migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c, db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path, false);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(c: &'m mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(c, migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c, db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path, false);
    }
    fn init(c: &'m mut Connection, db_path: &'m str, db_name: &'m str) -> Result<Migrator<'m>, String> {
        return Ok(Migrator { db_path, db_name, connection: c, skip_count: 0, });
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str, is_mem: bool) -> Result<usize, String> {
        if !is_mem {
            match self.attach(is_mem) {
                Ok(_) => {},
                Err(e) => return Err(format!("{}", e)),
            };
        }
        let res = self.up(mig_path);
        if !is_mem {
            match self.detach() {
                Ok(_) => {},
                Err(e) => return Err(format!("{}", e)),
            };
        }
        return res;
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str, is_mem: bool) -> Result<usize, String> {
        if !is_mem {
            match self.attach(is_mem) {
                Ok(_) => {},
                Err(e) => return Err(format!("{}", e)),
            };
        }
        let res = self.down(mig_path);
        if !is_mem {
            match self.detach() {
                Ok(_) => {},
                Err(e) => return Err(format!("{}", e)),
            };
        }
        return res;
    }
    #[cfg(test)]
    fn do_both<'a>(c: &'m mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        use std::fs::remove_file;
        match Migrator::run_from_memory(c, migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c, db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        let up_skips = match m.upward_migration(migrations_path, false) {
            Ok(up_skips) => up_skips,
            Err(e) => return Err(e),
        };
        let down_skips = match m.downward_migration(migrations_path, false) {
            Ok(down_skips) => down_skips,
            Err(e) => return Err(e),
        };
        match remove_file(&format!("{}/{}.db", db_path, db_name)) {
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
        const DB_PATH: &'static str = "./";
        const DB_NAME: &'static str = "Test";
        let mut c = match Connection::open(":memory:") {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };
        let skips = match Migrator::do_both(&mut c, "./test_sql", DB_PATH, DB_NAME) {
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

