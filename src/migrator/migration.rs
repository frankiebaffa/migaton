use {
    crate::migrator::{
        QuickMatch,
        MigratorError,
    },
    std::io::Read,
};
use super::super::{
    File,
    Path,
    PathBuf,
};
/// A database migration containing upward, downward, and check scripts
pub struct Migration {
    /// A number denoting the ordering of the migration
    pub number: i64,
    /// The file name
    pub file_name: String,
    /// An upward migration script
    pub up: String,
    /// A downward migration script
    pub down: String,
    /// A check migration script
    pub check: String,
}
#[derive(serde::Deserialize)]
struct ConfigFile {
    ordering: Vec<String>,
}
impl Migration {
    /// Creates a new migration
    fn new<'a>(
        number: i64, file_name: String, up: String, down: String,
        check: String
    ) -> Migration {
        return Migration { number, file_name, up, down, check, };
    }
    const UP_END: &'static str = "up.sql";
    const DOWN_END: &'static str = "down.sql";
    const CHK_END: &'static str = "chk.sql";
    const CONFIG_FILE: &'static str = "migaton.yml";
    /// Retrieves all migrations from the given path
    pub fn get_all(mig_path: String) -> Result<Vec<Migration>, MigratorError> {
        let p = PathBuf::from(mig_path.clone());
        if !p.is_dir() {
            return Err(MigratorError::Error(format!("Migration directory {} does not exist", mig_path)));
        }
        let config_file_path = PathBuf::from(format!("{}/{}", mig_path.clone(), Self::CONFIG_FILE));
        if !config_file_path.is_file() {
            return Err(MigratorError::Error(format!("Migration config file {} does not exist", format!("{}/{}", mig_path.clone(), Self::CONFIG_FILE))));
        }
        let mut config_file = File::open(config_file_path).quick_match()?;
        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string).quick_match()?;
        let config: ConfigFile = serde_yaml::from_str(config_string.as_str()).quick_match()?;
        let mut migrations: Vec<Migration> = Vec::new();
        let mut index = 0;
        for order in config.ordering {
            let up_file_name = format!("{}/{}/{}", mig_path, order, Self::UP_END);
            let up_exists = Path::new(&up_file_name).exists();
            let down_file_name = format!("{}/{}/{}", mig_path, order, Self::DOWN_END);
            let down_exists = Path::new(&down_file_name).exists();
            let chk_file_name = format!("{}/{}/{}", mig_path, order, Self::CHK_END);
            let chk_exists = Path::new(&chk_file_name).exists();
            if !up_exists || !down_exists || !chk_exists {
                return Err(MigratorError::Error(format!("Incomplete set for migration {}", order)));
            }
            let mut up_file = match File::open(&up_file_name) {
                Ok(up_file) => up_file,
                Err(e) => return Err(MigratorError::Error(format!("Failed to open file {}: {}", up_file_name, e))),
            };
            let mut up_script = String::new();
            match up_file.read_to_string(&mut up_script) {
                Ok(_) => {},
                Err(e) => return Err(MigratorError::Error(format!("Failed to read {} to string: {}", up_file_name, e))),
            };
            let mut down_file = match File::open(&down_file_name) {
                Ok(down_file) => down_file,
                Err(e) => return Err(MigratorError::Error(format!("Failed to open file {}: {}", down_file_name, e))),
            };
            let mut down_script = String::new();
            match down_file.read_to_string(&mut down_script) {
                Ok(_) => {},
                Err(e) => return Err(MigratorError::Error(format!("Failed to read {} to string: {}", down_file_name, e))),
            };
            let mut chk_file = match File::open(&chk_file_name) {
                Ok(chk_file) => chk_file,
                Err(e) => return Err(MigratorError::Error(format!("Failed to open file {}: {}", chk_file_name, e))),
            };
            let mut chk_script = String::new();
            match chk_file.read_to_string(&mut chk_script) {
                Ok(_) => {},
                Err(e) => return Err(MigratorError::Error(format!("Failed to read {} to string: {}", chk_file_name, e))),
            };
            migrations.push(Migration::new(index, order, up_script, down_script, chk_script));
            index = index + 1;
        }
        return Ok(migrations);
    }
}

