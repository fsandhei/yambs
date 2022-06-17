use crate::mmk_parser;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IncludeDirectories(Vec<IncludeDirectory>);

#[derive(Debug, PartialEq, Eq, Clone)]
struct IncludeDirectory {
    include_type: IncludeType,
    path: std::path::PathBuf,
}

impl IncludeDirectory {
    fn to_gnu_make_include(&self) -> String {
        match self.include_type {
            IncludeType::Include => format!("-I{}", self.path.display()),
            IncludeType::System => format!("-isystem {}", self.path.display()),
        }
    }
}

impl IncludeDirectories {
    pub fn from_mmk(mmk: &mmk_parser::Mmk) -> Option<Self> {
        if !mmk.has_dependencies() {
            return None;
        }
        let mut include_directories = vec![];
        for keyword in &mmk.data()["MMK_REQUIRE"] {
            let path_as_str = keyword.argument();
            let path = std::path::PathBuf::from(path_as_str);
            if keyword.option() == "SYSTEM" {
                include_directories.push(IncludeDirectory {
                    include_type: IncludeType::System,
                    path,
                });
            } else {
                include_directories.push(IncludeDirectory {
                    include_type: IncludeType::Include,
                    path,
                });
            }
        }
        Some(Self(include_directories))
    }

    pub fn to_gnu_make_include(&self) -> String {
        let mut gnu_make_include_str = String::new();
        for include_directory in &self.0 {
            let include_dir_str = include_directory.to_gnu_make_include();
            gnu_make_include_str.push_str(&include_dir_str);
        }

        gnu_make_include_str
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum IncludeType {
    Include,
    System,
}
