#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct CompilerFlags {
    #[serde(rename = "CXXFLAGS_APPEND")]
    cxx_flags: CXXFlags,
    #[serde(rename = "CPPFLAGS_APPEND")]
    cpp_flags: CPPFlags,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
struct CXXFlags(std::vec::Vec<String>);

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
struct CPPFlags(std::vec::Vec<String>);
