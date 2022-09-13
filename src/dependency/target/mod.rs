use std::cell::RefCell;
use std::rc::Rc;

use crate::flags::CompilerFlags;
use crate::targets;

mod associated_files;
mod include_directories;
mod target_registry;
use associated_files::AssociatedFiles;
use include_directories::IncludeDirectories;
use target_registry::TargetRegistry;

pub struct Target {
    path: std::path::PathBuf,
    modification_time: std::time::SystemTime,
    dependencies: Vec<TargetNode>,
    state: TargetState,
    associated_files: AssociatedFiles,
    target_type: TargetType,
    include_directories: Option<IncludeDirectories>,
    compiler_flags: CompilerFlags,
}

impl Target {
    pub fn executable(executable: targets::Executable, registry: &mut TargetRegistry) -> Self {
        unimplemented!()
    }
}

pub struct TargetNode(Rc<RefCell<Target>>);

pub enum TargetType {
    Executable(String),
    Library(String),
    None,
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetState {
    NotInProcess,
    InProcess,
    Registered,
    MakefileMade,
    Building,
    BuildComplete,
}

impl TargetState {
    pub fn new() -> Self {
        TargetState::NotInProcess
    }
}
