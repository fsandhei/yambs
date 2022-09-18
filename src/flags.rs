#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CompilerFlags {
    #[serde(rename = "CXXFLAGS_APPEND")]
    cxx_flags: CXXFlags,
    #[serde(rename = "CPPFLAGS_APPEND")]
    cpp_flags: CPPFlags,
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
struct CXXFlags(std::vec::Vec<String>);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
struct CPPFlags(std::vec::Vec<String>);
