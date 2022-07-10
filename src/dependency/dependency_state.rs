#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum DependencyState {
    NotInProcess,
    InProcess,
    Registered,
    MakefileMade,
    Building,
    BuildComplete,
}

impl DependencyState {
    pub fn new() -> Self {
        DependencyState::NotInProcess
    }
}
