use super::{
    Connection,
    ConnectionType,
    DoMigrations,
    MigratorAccess,
};
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
impl TestMigrator {
    /// Initializes a new TestMigrator object
    pub fn init<'b>(c_type: ConnectionType) -> Result<TestMigrator, String> {
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

