use std::io::Read;

#[derive(Debug)]
pub struct Progress {
    pub total: u64,
    pub current: u64,
    pub fh: std::fs::File,
    pub targets_to_build: Vec<std::path::PathBuf>,
}

impl Progress {
    pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
        let progress_file = path.join("progress.txt");

        let mut fh = std::fs::File::open(&progress_file)?;
        let mut buffer = Vec::new();

        fh.read_to_end(&mut buffer)?;

        let targets = String::from_utf8_lossy(&buffer)
            .split("\n")
            .map(|b| b.to_owned())
            .filter(|b| !b.is_empty())
            .map(std::path::PathBuf::from)
            .collect::<Vec<std::path::PathBuf>>();
        let total_targets = targets.len();

        let mut targets_built = 0;

        for target in &targets {
            if target.exists() {
                targets_built += 1;
            }
        }

        Ok(Self {
            total: total_targets as u64,
            current: targets_built,
            fh,
            targets_to_build: targets.clone(),
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
}
