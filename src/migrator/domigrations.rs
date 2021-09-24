use super::{
    ConnectionType,
    Migration,
    MigrationDirection,
    super::{
        Connection,
        Transaction,
    },
};
pub trait DoMigrations {
    fn get_skip_count(&mut self) -> usize;
    fn create_connection<'a>(c_type: ConnectionType) -> Result<Connection, String>;
    fn query_chk(c: &Connection, m: &Migration) -> Result<i64, String>;
    fn run_migration(t: &mut Transaction, m: &Migration, d: &MigrationDirection) -> Result<(), String>;
    fn migrate(&mut self, direction: MigrationDirection, mig_path: String) -> Result<usize, String>;
    fn up<'b>(&mut self, mig_path: &'b str) -> Result<usize, String>;
    fn down<'b>(&mut self, mig_path: &'b str) -> Result<usize, String>;
}

