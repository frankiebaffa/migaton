/// A representation of a limited migration
#[derive(PartialEq, Clone)]
pub enum MigrateTo {
    /// A limit
    StopAt(String),
    /// No limit
    NoStop,
}
