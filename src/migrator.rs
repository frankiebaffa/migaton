mod migrationdirection;
mod migration;
use {
    crate::{
        Connection,
        Transaction,
    },
    migrationdirection::MigrationDirection,
    migration::Migration,
    rusqlite::Error as RusqliteError,
    serde_yaml::Error as SerdeError,
    std::io::Error as IOError,
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
    pub fn run_from_memory<'a>(c: &mut Connection, migrations_path: &'a str) -> Result<usize, MigratorError> {
        let mut m = Migrator::init(c)?;
        let mut skips = m.upward_migration(migrations_path, true)?;
        skips = skips + m.downward_migration(migrations_path, true)?;
        if skips > 0 {
            return Err(MigratorError::Error(format!("Memory skips should always be 0, returned {}", skips)));
        }
        return Ok(skips);
    }
    /// Safely attempt to migrate upward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate upward on a copy of the DB file, then migrates the DB file
    pub fn do_up<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str) -> Result<usize, MigratorError> {
        Migrator::run_from_memory(mem_c, migrations_path)?;
        let mut m = Migrator::init(c)?;
        return m.upward_migration(migrations_path, false);
    }
    /// Safely attempt to migrate downward. Migrates up and down on a SQLite in-memory database, then
    /// attempts to migrate downward on a copy of the DB file, then migrates the DB file
    pub fn do_down<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str) -> Result<usize, MigratorError> {
        Migrator::run_from_memory(mem_c, migrations_path)?;
        let mut m = Migrator::init(c)?;
        return m.downward_migration(migrations_path, false);
    }
    fn init(c: &'m mut Connection) -> Result<Migrator<'m>, MigratorError> {
        return Ok(Migrator { connection: c, skip_count: 0, });
    }
    fn upward_migration<'b>(&mut self, mig_path: &'b str, is_test: bool) -> Result<usize, MigratorError> {
        return self.migrate(MigrationDirection::Up, mig_path.to_string(), is_test);
    }
    fn downward_migration<'b>(&mut self, mig_path: &'b str, is_test: bool) -> Result<usize, MigratorError> {
        return self.migrate(MigrationDirection::Down, mig_path.to_string(), is_test);
    }
    /// Runs the passed Migration's check script
    fn query_chk(c: &Connection, m: &Migration) -> Result<i64, MigratorError> {
        let mut chk_stmt = c.prepare(&m.check).quick_match()?;
        // TODO: change this back to a quick match.
        // returning 0 on error due to a bug in another program, checking how this works
        return match chk_stmt.query_row([], |row| row.get(0)) {
            Ok(i) => Ok(i),
            Err(_) => Ok(0),
        };
    }
    /// Runs the applicable script of a Migration based on the direction
    fn run_migration(t: &mut Transaction, m: &Migration, d: &MigrationDirection) -> Result<usize, MigratorError> {
        let s = match d {
            &MigrationDirection::Up => {
                m.up.clone()
            },
            &MigrationDirection::Down => {
                m.down.clone()
            },
        };
        return t.execute(&s, []).quick_match();
    }
    /// Migrates the SQLite database in the given direction
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String, is_test: bool) -> Result<usize, MigratorError> {
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
        let mut migrations = Migration::get_all(mig_path)?;
        let passing_int: i64;
        if direction.eq(&MigrationDirection::Down) {
            migrations.sort_unstable_by(|x, y| y.number.cmp(&x.number));
            passing_int = 1;
        } else {
            migrations.sort_unstable_by(|x, y| x.number.cmp(&y.number));
            passing_int = 0;
        }
        for migration in migrations {
            let passing_check = Self::query_chk(&self.access_connection(), &migration)?;
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
            let mut tx = self.access_connection().transaction().quick_match()?;
            Self::run_migration(&mut tx, &migration, &direction)?;
            tx.commit().quick_match()?;
        }
        return Ok(self.get_skip_count());
    }
    #[cfg(test)]
    fn do_both<'a>(mem_c: &mut Connection, c: &mut Connection, migrations_path: &'a str, db_path: &'a str, db_name: &'a str) -> Result<usize, MigratorError> {
        use std::fs::remove_file;
        Migrator::run_from_memory(mem_c, migrations_path)?;
        let mut m = Migrator::init(c)?;
        let up_skips = m.upward_migration(migrations_path, false)?;
        let down_skips = m.downward_migration(migrations_path, false)?;
        m.close();
        remove_file(&format!("{}/{}.db", db_path, db_name)).quick_match()?;
        return Ok(up_skips + down_skips);
    }
}
#[cfg(test)]
mod migrator_tests {
    use {
        crate::Migrator,
        worm::core::{DbCtx, DbContext},
        worm::derive::WormDb,
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

