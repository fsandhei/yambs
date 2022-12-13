pub mod command_line;
pub mod configurations;

use crate::errors::CommandLineError;

#[derive(Debug, Clone)]
pub struct BuildDirectory(std::path::PathBuf);

impl BuildDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}

impl std::convert::From<std::path::PathBuf> for BuildDirectory {
    fn from(f: std::path::PathBuf) -> Self {
        Self { 0: f }
    }
}

impl std::convert::From<&std::path::Path> for BuildDirectory {
    fn from(f: &std::path::Path) -> Self {
        Self { 0: f.to_path_buf() }
    }
}

impl Default for BuildDirectory {
    fn default() -> Self {
        Self {
            0: std::env::current_dir().expect("Could not locate current directory."),
        }
    }
}

impl std::string::ToString for BuildDirectory {
    fn to_string(&self) -> String {
        self.0.display().to_string()
    }
}

impl std::str::FromStr for BuildDirectory {
    type Err = CommandLineError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let canonicalized_path = canonicalize_path(&std::path::PathBuf::from(s))
            .map_err(crate::errors::FsError::Canonicalize)?;
        Ok(Self {
            0: canonicalized_path,
        })
    }
}

fn canonicalize_path(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    if !path.is_absolute() {
        return Ok(std::env::current_dir()?.join(path));
    }
    Ok(path.to_path_buf())
}
