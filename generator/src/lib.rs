mod include_file_generator;
mod generator;

pub use crate::generator::Generator;
pub use crate::include_file_generator::IncludeFileGenerator;

use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::path::PathBuf;

use dependency::DependencyNode;
use error::MyMakeError;
use utility;

#[allow(dead_code)]
pub struct MmkGenerator
{
    filename: Option<File>,
    dependency: DependencyNode,
    output_directory: std::path::PathBuf,
    debug: bool,
    include_file_generator: IncludeFileGenerator,
}


fn print_full_path(os: &mut String, dir: &str, filename: &str, no_newline: bool) {
    os.push_str(dir);
    os.push_str("/");
    os.push_str(filename);
    if !no_newline {
        os.push_str(" \\\n");
    }
}

impl MmkGenerator {
    pub fn new(dependency: &DependencyNode, build_directory: std::path::PathBuf) -> Result<MmkGenerator, MyMakeError> {
        let output_directory = build_directory;
        let include_output_directory = output_directory.join("make_include");
        utility::create_dir(&output_directory)?;
        
        Ok(MmkGenerator{ 
            filename: None, 
            dependency: dependency.clone(), 
            output_directory, 
            debug: false,
            include_file_generator: IncludeFileGenerator::new(&include_output_directory)
        })
    }


    pub fn replace_generator(&mut self, dependency: &DependencyNode, build_directory: std::path::PathBuf) {
        let gen = MmkGenerator::new(dependency, build_directory).unwrap();
        self.dependency       = gen.dependency;
        self.output_directory = gen.output_directory;
        let include_output_directory = self.output_directory.parent().unwrap().join("make_include");
        self.include_file_generator.change_directory(include_output_directory);
        self.create_makefile();
    }


    pub fn create_makefile(&mut self) {
        let filename = utility::create_file(&self.output_directory, "makefile").unwrap();
        self.filename = Some(filename);
    }


    fn use_subdir(&mut self, dir: std::path::PathBuf) -> Result<(), MyMakeError>{
        let new_output_dir = self.output_directory.join(dir);
        utility::create_dir(&new_output_dir)?;
        self.output_directory = new_output_dir;
        Ok(())
    }


    fn create_subdir(&self, dir: std::path::PathBuf) -> Result<(), MyMakeError> {
        utility::create_dir(&self.output_directory.join(dir))
    }

    fn get_required_project_lib_dir(&self) -> PathBuf {
        self.output_directory.join("libs")
    }

    #[allow(dead_code)]
    fn pop_dir(&mut self) {
        self.output_directory.pop();
    }


    // TODO: Make function return Result<String, MyMakeError>?
    pub fn make_object_rule(&self, mmk_data: &mmk_parser::Mmk) -> String {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.dependency.borrow();
        let mut object = String::new();

        if mmk_data.data.contains_key("MMK_SOURCES") {
            for source in &mmk_data.data["MMK_SOURCES"] {
                if let Some(source_path) = mmk_data.source_file_path(source) {
                    self.create_subdir(source_path).unwrap();
                }

                if source.ends_with(".cpp") {
                    object = source.replace(".cpp", ".o");
                }
                if source.ends_with(".cc") {
                    object = source.replace(".cc", ".o");
                }

                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&object);
                formatted_string.push_str(": \\\n");
                formatted_string.push_str("\t");
                formatted_string.push_str(borrowed_dependency.get_parent_directory().to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(source);
                formatted_string.push_str("\n");
                formatted_string.push_str(&format!("\t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) \
                                                          $(WARNINGS) {dependencies} $< -c -o $@)\n\n"
                , dependencies = self.print_dependencies()));
            }
        }
        formatted_string.trim_end().to_string()
    }


    fn print_header_includes(&self) -> String {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.dependency.borrow();
        let mmk_data = borrowed_dependency.mmk_data();
        let mut include_file = String::new();
        if mmk_data.data.contains_key("MMK_SOURCES") {
            for source in &mmk_data.data["MMK_SOURCES"] {
                if source.ends_with(".cpp") {
                    include_file = source.replace(".cpp", ".d");
                }
                if source.ends_with(".cc") {
                    include_file = source.replace(".cc", ".d");
                }
                
                formatted_string.push_str("sinclude ");
                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&include_file);
                formatted_string.push_str("\n");
            }
        }
        formatted_string
    }


    pub fn print_required_dependencies_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        for dependency in  self.dependency.borrow().requires().borrow().iter() {
            if dependency.borrow().library_name() != "" {
                let required_dep = dependency.borrow();
                let mut output_directory = self.get_required_project_lib_dir()
                                                  .join(required_dep.get_project_name());
                if self.debug {
                    output_directory = output_directory.join("debug");
                }
                else {
                    output_directory = output_directory.join("release");
                }
                formatted_string.push_str("\t");
                print_full_path(&mut formatted_string, 
                                output_directory.to_str().unwrap(),
                                &required_dep.library_name(),
                                false);
            }
        }
        formatted_string
    }


    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }


    fn print_library_name(&self) -> String {
        let mut formatted_string = String::new();
        print_full_path(&mut formatted_string,
                        self.output_directory.to_str().unwrap(),
                        &self.dependency.borrow().library_name(),
                        true);
        formatted_string
    }


    fn print_prerequisites(self: &Self) -> String {
        let mut formatted_string = String::new();
        let mut object = String::new();
        if self.dependency.borrow().mmk_data().data.contains_key("MMK_SOURCES") {
            formatted_string.push_str("\\\n");
            for source in &self.dependency.borrow().mmk_data().data["MMK_SOURCES"] {
                if source.ends_with(".cpp") {
                    object = source.replace(".cpp", ".o");
                }
                if source.ends_with(".cc") {
                    object = source.replace(".cc", ".o");
                }
                formatted_string.push_str("\t");
                print_full_path(&mut formatted_string,
                                self.output_directory.to_str().unwrap(),
                                &object,
                                false);
            }
        }
        formatted_string.push_str(&self.print_required_dependencies_libraries());
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        formatted_string
    }


    fn print_dependencies(&self) -> String {
        let mut formatted_string = self.dependency.borrow().mmk_data().get_include_directories();
        if self.dependency.borrow().mmk_data().has_system_include() {
            formatted_string.push_str(" ");
            formatted_string.push_str(&self.dependency.borrow().mmk_data().to_string("MMK_SYS_INCLUDE"));
        }

        formatted_string
    }


    pub fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), MyMakeError> {
        if !&dependency.borrow().is_makefile_made()
        {
            dependency.borrow_mut().makefile_made();
            self.generate_makefile()?;
        }

        let dependency_output_library_head = self.get_required_project_lib_dir();
        if !utility::directory_exists(&dependency_output_library_head) {
            utility::create_dir(&dependency_output_library_head)?;
        }

        for required_dependency in dependency.borrow().requires().borrow().iter()
        {
            if !required_dependency.borrow().is_makefile_made()
            {
                required_dependency.borrow_mut().makefile_made();
                let mut build_directory = dependency_output_library_head
                                                  .join(required_dependency.borrow().get_project_name());
                if self.debug {
                    build_directory.push("debug");
                }
                else {
                    build_directory.push("release");
                }
                self.replace_generator(&Rc::clone(required_dependency),
                                                 build_directory);
                self.generate_makefile()?;
            }
            self.generate_makefiles(&required_dependency)?;
        }
        Ok(())
    }


    pub fn debug(&mut self) {
        self.debug = true;
        self.use_subdir(std::path::PathBuf::from("debug")).unwrap();
    }


    pub fn release(&mut self) {
        if !self.debug {
            self.use_subdir(std::path::PathBuf::from("release")).unwrap();
        }
    }


    fn print_release(&self) -> String {
        let release_include = format!("{build_path}/release.mk",
        build_path = self.include_file_generator.print_build_directory());
        release_include
    }


    fn print_debug(&self) -> String {
        if self.debug {
            let debug_include = format!("{build_path}/debug.mk",
            build_path = self.include_file_generator.print_build_directory());
            debug_include
        }
        else {
            self.print_release()
        }
    }

    #[allow(dead_code)]
    fn print_build_directory(&self) -> &str {
        self.output_directory.to_str().unwrap()
    }


    pub fn use_std(&mut self, version: &str) -> Result<(), MyMakeError> {
        self.include_file_generator.add_cpp_version(version)
    }
}


impl Generator for MmkGenerator
{
    fn generate_makefile(self: &mut Self) -> Result<(), MyMakeError> {
        self.include_file_generator.generate_makefiles()?;
        self.create_makefile();
        self.generate_header()?;
        self.generate_appending_flags()?;
        if self.dependency.borrow().mmk_data().data.contains_key("MMK_EXECUTABLE") && 
           self.dependency.borrow().mmk_data().data["MMK_EXECUTABLE"] != {[""]}
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


    fn generate_header(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include {build_path}/strict.mk\n\
        include {build_path}/default_make.mk\n\
        include {debug}\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", 
        debug = self.print_debug(),
        build_path = self.include_file_generator.print_build_directory());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating header for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_package(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        {package}: {prerequisites}\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ", prerequisites = self.print_prerequisites()
         , package      = self.print_library_name()
         , sources_to_objects = self.make_object_rule(&self.dependency.borrow().mmk_data())
         , include_headers = self.print_header_includes());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating package rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: {executable}\n\
        {executable}: {prerequisites}\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) {dependencies} $^ -o $@)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ",
        executable         = self.dependency.borrow().mmk_data().to_string("MMK_EXECUTABLE"),
        prerequisites      = self.print_prerequisites(),
        dependencies       = self.print_dependencies(),
        sources_to_objects = self.make_object_rule(&self.dependency.borrow().mmk_data()),
        include_headers = self.print_header_includes());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_appending_flags(&mut self) -> Result<(), MyMakeError> {
        let mut data = String::new();

        if self.dependency.borrow().mmk_data().data.contains_key("MMK_CXXFLAGS_APPEND") {
            data.push_str(&format!("CXXFLAGS += {cxxflags}\n", 
            cxxflags = self.dependency.borrow().mmk_data().to_string("MMK_CXXFLAGS_APPEND")).to_owned());
        }

        if self.dependency.borrow().mmk_data().data.contains_key("MMK_CPPFLAGS_APPEND") {
            data.push_str(&format!("CPPFLAGS += {cppflags}\n", 
            cppflags = self.dependency.borrow().mmk_data().to_string("MMK_CPPFLAGS_APPEND")).to_owned());
        }

        if !data.is_empty() {
            match self.filename.as_ref().unwrap().write(data.as_bytes()) {
                Ok(_) => (),
                Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
            };
        }
        Ok(())
    }


    fn print_ok(self: &Self) -> () {
        print!(".");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::fs;
    use dependency::Dependency;
    use tempdir::TempDir;
    use pretty_assertions::assert_eq;


    #[allow(dead_code)]
    fn expected_library_name(path: &std::path::Path) -> String {
        let mut library_name = String::from("lib");
        library_name.push_str(utility::get_head_directory(path).to_str().unwrap());
        library_name.push_str(".a");
        library_name
    }

    #[test]
    fn generate_makefile_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let source_dir = dir.path().join("source");
        utility::create_dir(&source_dir).unwrap();
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        assert!(Generator::generate_makefile(&mut gen).is_ok());
        Ok(())
    }


    #[test]
    fn print_debug_test() -> std::io::Result<()> {
        let path = std::path::PathBuf::from("some_path");
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&path.join("run.mmk"))));
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.debug();
        assert_eq!(format!("{directory}/make_include/debug.mk",
                   directory = output_dir.path().to_str().unwrap()), gen.print_debug());
        Ok(())
    }


    #[test]
    fn generate_header_release_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!(format!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include {directory}/make_include/strict.mk\n\
        include {directory}/make_include/default_make.mk\n\
        include {directory}/make_include/release.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", directory = output_dir.path().to_str().unwrap()), 
        fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_header_debug_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();        
        gen.debug();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!(format!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include {directory}/make_include/strict.mk\n\
        include {directory}/make_include/default_make.mk\n\
        include {directory}/make_include/debug.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", directory = output_dir.path().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_package_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let source_dir = dir.path().join("source");
        utility::create_dir(&source_dir).unwrap();
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().add_library_name();
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);

        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");

        assert!(Generator::generate_rule_package(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        {directory}/libtmp.a: \\\n\
        \t{directory}/filename.o \\\n\
        \t{directory}/ofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {directory}/filename.o: \\\n\
        \t{dep_directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency/include -I/some/new/dependency/include $< -c -o $@)\n\
        \n\
        {directory}/ofilename.o: \\\n\
        \t{dep_directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency/include -I/some/new/dependency/include $< -c -o $@)\n\
        \n\
        sinclude {directory}/filename.d\n\
        sinclude {directory}/ofilename.d\n\
        \n", 
        directory = output_dir.path().to_str().unwrap(),
        dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_executable_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let source_dir = dir.path().join("source");
        utility::create_dir(&source_dir).unwrap();
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_EXECUTABLE".to_string(), vec!["x".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_rule_executable(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: x\n\
        x: \\\n\
        \t{directory}/filename.o \\\n\
        \t{directory}/ofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency/include -I/some/new/dependency/include $^ -o $@)\n\
        \n\
        {directory}/filename.o: \\\n\
        \t{dep_directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency/include -I/some/new/dependency/include $< -c -o $@)\n\
        \n\
        {directory}/ofilename.o: \\\n\
        \t{dep_directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency/include -I/some/new/dependency/include $< -c -o $@)\n\
        \n\
        sinclude {directory}/filename.d\n\
        sinclude {directory}/ofilename.d\n\
        \n",
        directory = output_dir.path().to_str().unwrap(),
        dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test_cxxflags() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CXXFLAGS += -pthread\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test_cppflags() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CPPFLAGS += -somesetting\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);

        let mut gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CXXFLAGS += -pthread\n\
        CPPFLAGS += -somesetting\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn print_header_includes_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        let gen = MmkGenerator::new(&dependency, output_dir.path().to_path_buf()).unwrap();
        let actual = gen.print_header_includes();
        let expected = format!("sinclude {directory}/filename.d\n\
                                       sinclude {directory}/ofilename.d\n",
                                       directory = output_dir.path().to_str().unwrap());
        assert_eq!(actual, expected);
        Ok(())
    }
}
