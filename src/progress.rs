use crate::generator;

pub const PROGRESS_FILE_NAME: &str = "progress.json";

#[derive(Debug)]
pub struct Progress {
    pub total: u64,
    pub current: u64,
    pub targets_to_build: Vec<std::path::PathBuf>,
}

impl Progress {
    pub fn new(path: &std::path::Path, target: Option<String>) -> std::io::Result<Self> {
        let progress_file = path.join(PROGRESS_FILE_NAME);

        let fh = std::fs::File::open(progress_file)?;
        let reader = std::io::BufReader::new(fh);

        let progress_document: generator::targets::ProgressDocument =
            serde_json::from_reader(reader)?;
        let targets = progress_document.targets;

        let object_files = if let Some(target) = target {
            Progress::object_files_from_target(&targets, &target)
        } else {
            Progress::object_files_from_target(&targets, "all")
        };

        let total = object_files.len() as u64;

        let mut object_files_built = 0;

        for object_file in &object_files {
            if object_file.exists() {
                object_files_built += 1;
            }
        }
        Ok(Self {
            total,
            current: object_files_built,
            targets_to_build: object_files,
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        let mut targets_built = 0;

        for target in &self.targets_to_build {
            if std::path::Path::new(&target).exists() {
                targets_built += 1;
            }
        }
        self.current = targets_built;
        Ok(())
    }

    fn object_files_from_target(
        targets: &[generator::targets::ProgressTrackingTarget],
        target: &str,
    ) -> Vec<std::path::PathBuf> {
        let progress_target = targets.iter().find(|t| t.target == target).unwrap();
        let mut object_files = Vec::<std::path::PathBuf>::new();
        for dependency in &progress_target.dependencies {
            let target_dependency = targets.iter().find(|t| t.target == *dependency).unwrap();

            for object_file in &target_dependency.object_files {
                if !object_files.contains(object_file) {
                    object_files.push(object_file.to_owned());
                }
            }
        }
        object_files.extend_from_slice(&progress_target.object_files);
        object_files
    }
}
