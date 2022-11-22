use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

use crate::build_target::target_registry::TargetRegistry;
use crate::build_target::{DependencySource, TargetNode, TargetSource};

pub fn dottie(
    top: &TargetNode,
    registry: &TargetRegistry,
    recursive: bool,
    data: &mut String,
) -> std::io::Result<()> {
    let mut dottie_file = create_dottie_file(recursive)?;
    let borrowed_top = top.borrow();
    let top_pretty_name = &borrowed_top.name();

    if !recursive {
        data.push_str(
            "\
        digraph G {\n\
        ",
        );
        dottie(top, registry, true, data)?;
        data.push('}');
        dottie_file.write_all(data.as_bytes())?;
    }

    match borrowed_top.target_source {
        TargetSource::FromSource(ref s) => {
            for requirement in &s.dependencies {
                match requirement.source {
                    DependencySource::FromSource(ref ds) => {
                        data.push_str(&format!(
                            "\
                            {:?} -> {:?}\n\
                            ",
                            ds.name, top_pretty_name
                        ));
                    }
                    DependencySource::FromPrebuilt(ref b) => {
                        data.push_str(&format!(
                            "\
                            {:?} -> {:?}\n\
                            ",
                            b.name, top_pretty_name
                        ));
                    }
                }
                dottie(
                    &requirement.to_build_target(registry).unwrap(),
                    registry,
                    true,
                    data,
                )?;
            }
        }
        TargetSource::FromPrebuilt(_) => {}
    }
    Ok(())
}

fn create_dottie_file(first_run: bool) -> std::io::Result<File> {
    let current_dir = env::current_dir()?;
    let dot_file_path = current_dir.join("dependency.gv");

    if dottie_file_exists() {
        if !first_run {
            File::create(dot_file_path)
        } else {
            OpenOptions::new()
                .write(true)
                .append(true)
                .open(dot_file_path)
        }
    } else {
        File::create(dot_file_path)
    }
}

fn dottie_file_exists() -> bool {
    let current_dir = env::current_dir().expect("rsmake: Current path does not exist!");
    let dot_file_path = current_dir.join("dependency.gv");
    dot_file_path.exists()
}
