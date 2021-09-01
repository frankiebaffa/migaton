/// A representation of the type of SQLite connection being opened
#[derive(PartialEq, Clone)]
pub enum ConnectionType<'a> {
    /// A connection to a temporary, in-memory SQLite database
    Memory,
    /// A connection to a SQLite database file
    DbFile(&'a str),
    /// A connection to a SQLite database file alongside a copy of said db file
    SafeDbFile(&'a str, &'a str),
}

