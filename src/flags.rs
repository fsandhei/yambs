#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CompilerFlags {
    #[serde(rename = "cxxflags_append")]
    pub cxx_flags: CXXFlags,
    #[serde(rename = "cppflags_append")]
    pub cpp_flags: CPPFlags,
}

impl CompilerFlags {
    pub fn new() -> Self {
        Self {
            cxx_flags: CXXFlags(Vec::new()),
            cpp_flags: CPPFlags(Vec::new()),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CXXFlags(std::vec::Vec<String>);

impl CXXFlags {
    pub fn flags(&self) -> &std::vec::Vec<String> {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CPPFlags(std::vec::Vec<String>);

impl CPPFlags {
    pub fn flags(&self) -> &std::vec::Vec<String> {
        &self.0
    }
}
