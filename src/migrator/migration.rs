use super::super::{
    File,
    Path,
    PathBuf,
    Read,
};
/// A database migration containing upward, downward, and check scripts
pub struct Migration {
    /// A number denoting the ordering of the migration
    pub number: i64,
    /// An upward migration script
    pub up: String,
    /// A downward migration script
    pub down: String,
    /// A check migration script
    pub check: String,
}
impl Migration {
    /// Creates a new migration
    fn new<'a>(number: i64, up: String, down: String, check: String) -> Migration {
        return Migration { number, up, down, check, };
    }
    const UP_END: &'static str = "up.sql";
    const DOWN_END: &'static str = "down.sql";
    const CHK_END: &'static str = "chk.sql";
    /// Retrieves all migrations from the given path
    pub fn get_all(mig_path: String) -> Result<Vec<Migration>, String> {
        let p = PathBuf::from(mig_path.clone());
        if !p.is_dir() {
            return Err(format!("Migration directory {} does not exist", mig_path));
        }
        let mut migrations: Vec<Migration> = Vec::new();
        let mut index = 1;
        loop {
            let up_file_name = format!("{}/{}.{}", mig_path, index, Self::UP_END);
            let up_exists = Path::new(&up_file_name).exists();
            let down_file_name = format!("{}/{}.{}", mig_path, index, Self::DOWN_END);
            let down_exists = Path::new(&down_file_name).exists();
            let chk_file_name = format!("{}/{}.{}", mig_path, index, Self::CHK_END);
            let chk_exists = Path::new(&chk_file_name).exists();
            if !up_exists && !down_exists && !chk_exists {
                return Ok(migrations);
            } else if !up_exists || !down_exists || !chk_exists {
                return Err(format!("Incomplete set for migration {}", index));
            }
            let mut up_file = match File::open(&up_file_name) {
                Ok(up_file) => up_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", up_file_name, e)),
            };
            let mut up_script = String::new();
            match up_file.read_to_string(&mut up_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", up_file_name, e)),
            };
            let mut down_file = match File::open(&down_file_name) {
                Ok(down_file) => down_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", down_file_name, e)),
            };
            let mut down_script = String::new();
            match down_file.read_to_string(&mut down_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", down_file_name, e)),
            };
            let mut chk_file = match File::open(&chk_file_name) {
                Ok(chk_file) => chk_file,
                Err(e) => return Err(format!("Failed to open file {}: {}", chk_file_name, e)),
            };
            let mut chk_script = String::new();
            match chk_file.read_to_string(&mut chk_script) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read {} to string: {}", chk_file_name, e)),
            };
            migrations.push(Migration::new(index, up_script, down_script, chk_script));
            index = index + 1;
        }
    }
}

