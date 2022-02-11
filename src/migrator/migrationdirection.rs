use crate::migrator::migrateto::MigrateTo;
/// A representation of the direction of a migration
#[derive(PartialEq, Clone)]
pub enum MigrationDirection {
    /// An upward migration
    Up(MigrateTo),
    /// A downward migration
    Down(MigrateTo),
}

