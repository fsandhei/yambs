use std::path::PathBuf;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CompilerFlags {
    #[serde(rename = "cxxflags_append")]
    pub cxx_flags: Option<CXXFlags>,
    #[serde(rename = "cflags_append")]
    pub c_flags: Option<CFlags>,
    #[serde(rename = "cppflags_append")]
    pub cpp_flags: Option<CPPFlags>,
    #[serde(rename = "ldflags_append")]
    pub ld_flags: Option<LDFlags>,
    #[serde(rename = "append_include_directories", default = "Vec::new")]
    pub include_directories: Vec<PathBuf>,
    #[serde(rename = "append_system_include_directories", default = "Vec::new")]
    pub system_include_directories: Vec<PathBuf>,
}

impl CompilerFlags {
    pub fn new() -> Self {
        Self {
            c_flags: None,
            cxx_flags: None,
            cpp_flags: None,
            ld_flags: None,
            include_directories: Vec::new(),
            system_include_directories: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CFlags(std::vec::Vec<String>);

impl CFlags {
    pub fn from_slice(flags: &[String]) -> Self {
        Self(flags.to_vec())
    }

    pub fn new(flags: &[&str]) -> Self {
        let flags = flags.iter().map(|f| f.to_string()).collect::<Vec<String>>();
        Self(flags)
    }

    pub fn flags(&self) -> &std::vec::Vec<String> {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CXXFlags(std::vec::Vec<String>);

impl CXXFlags {
    pub fn from_slice(flags: &[String]) -> Self {
        Self(flags.to_vec())
    }

    pub fn new(flags: &[&str]) -> Self {
        let flags = flags.iter().map(|f| f.to_string()).collect::<Vec<String>>();
        Self(flags)
    }

    pub fn flags(&self) -> &std::vec::Vec<String> {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CPPFlags(Vec<String>);

impl CPPFlags {
    pub fn from_slice(flags: &[String]) -> Self {
        Self(flags.to_vec())
    }

    pub fn flags(&self) -> &std::vec::Vec<String> {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct LDFlags(std::vec::Vec<String>);

impl LDFlags {
    pub fn from_slice(flags: &[String]) -> Self {
        Self(flags.to_vec())
    }

    pub fn new(flags: &[&str]) -> Self {
        let flags = flags.iter().map(|f| f.to_string()).collect::<Vec<String>>();
        Self(flags)
    }

    pub fn flags(&self) -> &Vec<String> {
        &self.0
    }
}
