use thiserror;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MyMakeError {
    #[error("Error occured during compilation: {description}")]
    CompileTime {
        description: String
    },
    #[error("Error occured during configure time: {description}")]
    ConfigurationTime {
        description: String
    },
    #[error("{description}")]
    Generic {
        description: String
    },
    
    #[error("Error occured during parsing of file {file}: {description}")]
    Parse {
        file: std::path::PathBuf,
        description: String,
    },
}


#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Error occured in creating directory {0:?}")]
    CreateDirectory(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured in removing directory {0:?}")]
    RemoveDirectory(std::path::PathBuf, #[source] std::io::Error),
    #[error("Failed to create symlink between {dest:?} and {src:?}")]
    CreateSymlink {
        dest: std::path::PathBuf,
        src: std::path::PathBuf,
        #[source] 
        source: std::io::Error
    },
    #[error("Error occured in removing file {0:?}")]
    RemoveFile(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured in creating file {0:?}")]
    CreateFile(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured reading from file {0:?}")]
    ReadFromFile(std::path::PathBuf, #[source] std::io::Error),
}
// impl MyMakeError {
//     pub fn from_str(msg: &str) -> MyMakeError {
//         MyMakeError{details: msg.to_string()}
//     }
//     pub fn from(msg: String) -> MyMakeError {
//         MyMakeError{details: msg}
//     }

//     pub fn to_string(&self) -> &String {
//         &self.details
//     }
// }

// impl fmt::Display for MyMakeError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.details)
//     }
// }

