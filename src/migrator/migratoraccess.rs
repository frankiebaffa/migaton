use super::{
    ConnectionType,
    DoMigrations,
    MigrationDirection,
    Migration,
    super::{
        Connection,
        Transaction,
    },
};
pub trait MigratorAccess {
    fn access_skip_count(&mut self) -> &mut usize;
    fn access_connection(&mut self) -> &mut Connection;
    fn inc_skip_count(&mut self);
}
impl<T> DoMigrations for T where T: MigratorAccess {
    /// Creates a connection to a SQLite database
    fn create_connection<'a>(c_type: ConnectionType) -> Result<Connection, String> {
        let db_file = c_type.get_full_db_path();
        let conn = match rusqlite::Connection::open(&db_file) {
            Ok(c) => c,
            Err(e) => return Err(format!("Failed to open connection to {}: {}", db_file, e)),
        };
        let db_name = c_type.get_db_name();
        let attach = format!("attach '{}' as {}", c_type.get_full_db_path(), db_name);
        match conn.execute(attach.as_str(), []) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        };
        return Ok(conn);
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
