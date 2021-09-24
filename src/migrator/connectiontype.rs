/// A representation of the type of SQLite connection being opened
#[derive(PartialEq, Clone)]
pub enum ConnectionType<'a> {
    /// A connection to a temporary, in-memory SQLite database
    Memory(&'a str),
    /// A connection to a SQLite database file
    DbFile(&'a str, &'a str),
}
impl<'a> ConnectionType<'a> {
    pub fn get_db_name(&self) -> &'a str {
        match &self {
            Self::Memory(db_name) => db_name,
            Self::DbFile(_, db_name) => db_name,
        }
    }
    //pub fn get_db_file_name(&self) -> String {
    //    match &self {
    //        Self::Memory(db_name) => db_name.to_string(),
    //        Self::DbFile(_, db_name) | Self::SafeDbFile(_, db_name) => format!("{}.db", db_name),
    //    }
    //}
    pub fn get_full_db_path(&self) -> String {
        match &self {
            Self::Memory(_) => return String::from(":memory:"),
            Self::DbFile(db_path, db_name) => {
                if db_path.ends_with("/") {
                    return format!("{}{}.db", db_path, db_name);
                } else {
                    return format!("{}/{}.db", db_path, db_name);
                }
            },
        }
    }
}

