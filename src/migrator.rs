use super::Connection;
use super::Transaction;
mod migrationdirection;
use migrationdirection::MigrationDirection;
mod migration;
use migration::Migration;
/// A higher-level object to manage migration statuses
pub struct Migrator<'m> {
    /// The db path
    db_path: &'m str,
    /// The db name
    db_name: &'m str,
    /// A connection to a SQLite database
    connection: Connection,
    /// A count of skipped migrations
    skip_count: usize,
}
impl<'m> Migrator<'m> {
    pub fn close(self) {
        drop(self.connection);
        return;
    }
    fn inc_skip_count(&mut self) {
        self.skip_count = self.skip_count + 1;
    }
    fn access_connection(&mut self) -> &mut Connection {
        return &mut self.connection;
    }
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
    pub fn run_from_memory<'a>(migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(db_path, db_name) {
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
        match m.connection.close() {
            Ok(_) => {},
            Err((_, e)) => return Err(format!("{}", e)),
        }
        if skips > 0 {
            return Err(format!("Memory skips should always be 0, returned {}", skips));
        }
        return Ok(skips);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path, false);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(db_path, db_name) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path, false);
    }
    fn init(db_path: &'m str, db_name: &'m str) -> Result<Migrator<'m>, String> {
        let c = match Connection::open("") {
            Ok(c) => c,
            Err(e) => return Err(format!("{}", e)),
        };
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
            let do_run = if direction.eq(&MigrationDirection::Down) && passing_check.ge(&passing_int) {
                true
            } else if direction.eq(&MigrationDirection::Up) && passing_check.eq(&passing_int) {
                true
            } else {
                false
            };
            if !do_run {
                self.inc_skip_count();
                println!("Skipping {}. Result {}", migration.file_name, passing_check);
                continue;
            }
            println!("Running '{}'", migration.file_name);
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
    #[cfg(test)]
    fn do_both<'a>(migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        use std::fs::remove_file;
        match Migrator::run_from_memory(migrations_path, db_path, db_name) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(db_path, db_name) {
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
        m.close();
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
    #[test]
    fn t_file_up_down() {
        const DB_PATH: &'static str = "./";
        const DB_NAME: &'static str = "Test";
        let skips = match Migrator::do_both("./test_sql", DB_PATH, DB_NAME) {
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

