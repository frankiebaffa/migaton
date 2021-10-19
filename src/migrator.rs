use super::Connection;
use super::Transaction;
mod migrationdirection;
use migrationdirection::MigrationDirection;
mod migration;
use migration::Migration;
/// A higher-level object to manage migration statuses
pub struct Migrator<'m> {
    /// A connection to a SQLite database
    connection: &'m mut Connection,
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
    /// Migrate upwards and downwards on a database stored in memory (used for testing)
    pub fn run_from_memory<'a>(c: &mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        let mut skips = match m.upward_migration(migrations_path, true) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        skips = skips + match m.downward_migration(migrations_path, true) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        if skips > 0 {
            return Err(format!("Memory skips should always be 0, returned {}", skips));
        }
        return Ok(skips);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(mem_c, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.upward_migration(migrations_path, false);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str) -> Result<usize, String> {
        match Migrator::run_from_memory(mem_c, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };
        return m.downward_migration(migrations_path, false);
    }
    fn init(c: &'m mut Connection) -> Result<Migrator<'m>, String> {
        return Ok(Migrator { connection: c, skip_count: 0, });
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str, is_test: bool) -> Result<usize, String> {
        return self.migrate(MigrationDirection::Up, mig_path.to_string(), is_test);
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str, is_test: bool) -> Result<usize, String> {
        return self.migrate(MigrationDirection::Down, mig_path.to_string(), is_test);
    }
    /// Runs the passed Migration's check script
    fn query_chk(c: &Connection, m: &Migration) -> Result<i64, String> {
        println!("{}", &m.check);
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
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String, is_test: bool) -> Result<usize, String> {
        let run_verb = if is_test {
            "Testing"
        } else {
            "Running"
        };
        let dir_str = match direction {
            MigrationDirection::Up => {
                "up"
            },
            MigrationDirection::Down => {
                "down"
            },
        };
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
            println!("{} {} '{}'", run_verb, dir_str, migration.file_name);
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
    #[cfg(test)]
    fn do_both<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, String> {
        use std::fs::remove_file;
        match Migrator::run_from_memory(mem_c, migrations_path) {
            Ok(_) => {},
            Err(e) => return Err(e),
        };
        let mut m = match Migrator::init(c) {
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
    use {
        crate::Migrator,
        worm::{DbCtx, DbContext},
        worm_derive::WormDb,
    };
    #[derive(WormDb)]
    #[db(var(name="WORMDBS"))]
    struct TestDb {
        context: DbContext,
    }
    #[test]
    fn t_file_up_down() {
        const DB_PATH: &'static str = "./";
        const DB_NAME: &'static str = "Test";
        let mut mem_testdb = TestDb::init();
        mem_testdb.context.attach_temp_dbs();
        let mut testdb = TestDb::init();
        testdb.context.attach_dbs();
        let mut mem_c = mem_testdb.context.use_connection();
        let mut c = testdb.context.use_connection();
        let skips = match Migrator::do_both(&mut mem_c, &mut c, "./test_sql", DB_PATH, DB_NAME) {
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

