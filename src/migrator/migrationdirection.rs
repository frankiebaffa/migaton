/// A representation of the direction of a migration
#[derive(PartialEq)]
pub enum MigrationDirection {
    /// An upward migration
    Up,
    /// A downward migration
    Down,
}

