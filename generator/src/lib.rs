
use std::fs::File;
use std::io::Write;

use dependency::Dependency;
use error::MyMakeError;
#[allow(dead_code)]
pub struct MmkGenerator
{
    filename: File,
    dependency: Dependency,
    output_directory: std::path::PathBuf,
}

impl MmkGenerator
{
    pub fn new(dependency: &Dependency, build_directory: std::path::PathBuf) -> Result<MmkGenerator, MyMakeError>
    {
        let output_directory = dependency.path().parent().unwrap().join(&build_directory);
        if !output_directory.is_dir() {
            match std::fs::create_dir(&output_directory) {
                Ok(()) => (),
                Err(err) => return Err(MyMakeError::from(format!("Error creating {:?}: {}", output_directory, err))),
            };
        }

        let output_file = &output_directory.join("makefile");
        if output_file.is_file() {
            match std::fs::remove_file(&output_file) {
                Ok(()) => (),
                Err(err) => return Err(MyMakeError::from(format!("Error removing {:?}: {}", output_file, err))),
            };
        }
        let filename = File::create(&output_directory
                                        .join("makefile"))
                                        .expect("Something went wrong");
        Ok(MmkGenerator{ filename: filename, dependency: dependency.clone(), output_directory: output_directory})
    }

    pub fn make_object_rule(self: &Self, mmk_data: &mmk_parser::Mmk) -> String {
        let mut formatted_string = String::new();
        let parent_path = &self.dependency.path().parent().unwrap();
        if mmk_data.data.contains_key("MMK_SOURCES") {
            for source in &mmk_data.data["MMK_SOURCES"] {
                let object = source.replace(".cpp", ".o");
                formatted_string.push_str(&object);
                formatted_string.push_str(": \\\n");
                formatted_string.push_str("\t");
                formatted_string.push_str(parent_path.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(source);

                if mmk_data.data.contains_key("MMK_HEADERS")
                && mmk_data.data["MMK_HEADERS"].first() != Some(&"".to_string()) {
                    formatted_string.push_str(" \\\n");
                    for header in &mmk_data.data["MMK_HEADERS"] {                        
                        formatted_string.push_str("\t");
                        formatted_string.push_str(parent_path.to_str().unwrap());
                        formatted_string.push_str("/");
                        formatted_string.push_str(header);
                        if Some(header) == mmk_data.data["MMK_HEADERS"].last() {
                            formatted_string.push_str("\n");
                        }
                        else {
                            formatted_string.push_str(" \\\n");
                        }
                    }
                } 
                else {
                    formatted_string.push_str("\n");
                }
                formatted_string.push_str(&format!("\t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) {dependencies} -I{path_str} $< -c -o $@)\n\n"
                , dependencies = mmk_data.to_string("MMK_DEPEND")
                , path_str = parent_path.to_str().unwrap()));
            }
        }
        formatted_string.trim_end().to_string()
    }
    pub fn print_required_dependencies_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        for dependency in  self.dependency.requires().borrow().iter() {            
            if dependency.borrow().library_name() != "" {
                let required_dep = dependency.borrow();
                let parent_path = required_dep.path().parent().unwrap();
                let build_path = parent_path.join(".build");
                formatted_string.push_str("\t");
                formatted_string.push_str(build_path.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&required_dep.library_name());
                formatted_string.push_str(" \\\n");            
            }
        }
        formatted_string
    }

    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }

    pub fn print_prerequisites(self: &Self) -> String {
        let mut formatted_string = String::new();
        if self.dependency.mmk_data().data.contains_key("MMK_SOURCES") {
            formatted_string.push_str("\\\n");
            for source in &self.dependency.mmk_data().data["MMK_SOURCES"] {
                let object = source.replace(".cpp", ".o");
                formatted_string.push_str("\t");
                formatted_string.push_str(&object);
                formatted_string.push_str(" \\\n");
            }
        }
        formatted_string.push_str(&self.print_required_dependencies_libraries());
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        formatted_string
    }
}

pub trait Generator
{      
    fn generate_makefile(self: &mut Self)        -> Result<(), MyMakeError>;
    fn generate_header(self: &mut Self)          -> Result<(), MyMakeError>;
    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError>;
    fn generate_rule_package(self: &mut Self)    -> Result<(), MyMakeError>;
    fn print_ok(self: &Self);    
}

impl Generator for MmkGenerator
{
    fn generate_makefile(self: &mut Self) -> Result<(), MyMakeError>
    {
        self.generate_header()?;
        if self.dependency.mmk_data().data.contains_key("MMK_EXECUTABLE") && 
           self.dependency.mmk_data().data["MMK_EXECUTABLE"] != {[""]}
        {
            self.generate_rule_executable()?;
        }
        else
        {
            self.generate_rule_package()?;
        }
        self.print_ok();
        Ok(())
    }

    fn generate_header(self: &mut Self) -> Result<(), MyMakeError>
    {
        match self.filename.write(b"\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n") {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating header for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_package(self: &mut Self) -> Result<(), MyMakeError>
    {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        .PHONY: {package}\n\
        {package}: {prerequisites}\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {sources_to_objects}\n\
        ", prerequisites = self.print_prerequisites()
         , package      = self.dependency.library_name()
         , sources_to_objects = self.make_object_rule(&self.dependency.mmk_data()));
        
        match self.filename.write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating package rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError>
    {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: {executable}\n\
        {executable}: {prerequisites}\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) {dependencies} $^ -o $@)\n\
        \n\
        {sources_to_objects}\n\
        ",
        executable         = self.dependency.mmk_data().to_string("MMK_EXECUTABLE"),
        prerequisites      = self.print_prerequisites(),
        dependencies       = self.dependency.mmk_data().to_string("MMK_DEPEND"),
        sources_to_objects = self.make_object_rule(&self.dependency.mmk_data()));
        
        match self.filename.write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn print_ok(self: &Self) -> ()
    {
        print!(".");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_generate_makefile() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.mmk_data.data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir).unwrap();
        assert!(Generator::generate_makefile(&mut gen).is_ok());
        Ok(())
    }
    #[test]
    fn test_generate_header() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.mmk_data.data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir).unwrap();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }
    #[test]
    fn test_generate_package() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.add_library_name();
        dependency.mmk_data.data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);

        let mut gen = MmkGenerator::new(&dependency, output_dir).unwrap();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_rule_package(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        .PHONY: libpkg.a\n\
        libpkg.a: \\\n\
        \tfilename.o \\\n\
        \tofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        filename.o: \\\n\
        \t{directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        ofilename.o: \\\n\
        \t{directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        ", directory = dir.path().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }

    #[test]
    fn test_generate_executable() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.mmk_data.data.insert("MMK_EXECUTABLE".to_string(), vec!["x".to_string()]);
        dependency.mmk_data.data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir).unwrap();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_rule_executable(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: x\n\
        x: \\\n\
        \tfilename.o \\\n\
        \tofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency $^ -o $@)\n\
        \n\
        filename.o: \\\n\
        \t{directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        ofilename.o: \\\n\
        \t{directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        ", directory = dir.path().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }
}
