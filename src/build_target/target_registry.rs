use crate::build_target::{BuildTarget, TargetNode};
// LEGG TIL TESTER

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TargetRegistry {
    pub registry: Vec<TargetNode>,
}

impl TargetRegistry {
    pub fn new() -> TargetRegistry {
        TargetRegistry {
            registry: Vec::new(),
        }
    }

    pub fn number_of_targets(&self) -> usize {
        self.registry.len()
    }

    pub fn add_target(&mut self, target: TargetNode) {
        self.registry.push(target);
    }

    pub fn get_target_from_predicate<P>(&self, predicate: P) -> Option<TargetNode>
    where
        P: Fn(&BuildTarget) -> bool,
    {
        for target in &self.registry {
            let borrowed_target = target.try_borrow();
            if let Ok(borrowed_target) = borrowed_target {
                if predicate(&borrowed_target) {
                    return Some(target.clone());
                }
            }
        }
        None
    }
}
