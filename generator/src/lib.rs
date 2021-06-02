mod include_file_generator;
mod generator;
pub mod generator_mock;

pub use crate::generator::Generator;
// pub use crate::include_file_generator::IncludeFileGenerator;

use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::path::PathBuf;

use dependency::DependencyNode;
use error::MyMakeError;
use utility;
use include_file_generator::IncludeFileGenerator;

#[allow(dead_code)]
pub struct MakefileGenerator
{
    filename: Option<File>,
    dependency: Option<DependencyNode>,
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

impl MakefileGenerator {
    pub fn new(build_directory: std::path::PathBuf) -> MakefileGenerator {
        let output_directory = build_directory;
        let include_output_directory = output_directory.join("make_include");
        utility::create_dir(&output_directory).unwrap();

        MakefileGenerator{ 
            filename: None, 
            dependency: None, 
            output_directory, 
            debug: false,
            include_file_generator: IncludeFileGenerator::new(&include_output_directory)
        }
    }

    // Må testes
    pub fn replace_generator(&mut self, dependency: &DependencyNode, build_directory: std::path::PathBuf) {
        let gen = MakefileGenerator::new(build_directory);
        self.set_dependency(dependency);
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


    fn make_object_rule(&self, mmk_data: &mmk_parser::Mmk) -> Result<String, MyMakeError> {
        let mut formatted_string = String::new();
        
        let borrowed_dependency = self.get_dependency()?.borrow();

        if mmk_data.data().contains_key("MMK_SOURCES") {
            let mut object = String::new();
            for source in &mmk_data.data()["MMK_SOURCES"] {
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
                , dependencies = self.print_dependencies()?));
            }
        }
        Ok(formatted_string.trim_end().to_string())
    }


    fn print_header_includes(&self) -> Result<String, MyMakeError> {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.get_dependency()?.borrow();
        let mmk_data = borrowed_dependency.mmk_data();
        let mut include_file = String::new();
        if mmk_data.data().contains_key("MMK_SOURCES") {
            for source in &mmk_data.data()["MMK_SOURCES"] {
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
        Ok(formatted_string)
    }


    fn print_required_dependencies_libraries(&self) -> Result<String, MyMakeError> {
        let mut formatted_string = String::new();
        for dependency in  self.get_dependency()?.borrow().requires().borrow().iter() {
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
                                &required_dep.library_file_name(),
                                false);
            }
        }
        Ok(formatted_string)
    }


    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }


    fn print_library_name(&self) -> Result<String, MyMakeError> {
        let mut formatted_string = String::new();
        print_full_path(&mut formatted_string,
                        self.output_directory.to_str().unwrap(),
                        &self.get_dependency()?.borrow().library_file_name(),
                        true);
        
        Ok(formatted_string)
    }


    fn print_prerequisites(self: &Self) ->Result<String, MyMakeError> {
        let mut formatted_string = String::new();
        let mut object = String::new();
        let borrowed_dependency = self.get_dependency()?.borrow();
        if borrowed_dependency.mmk_data().data().contains_key("MMK_SOURCES") {
            formatted_string.push_str("\\\n");
            for source in &borrowed_dependency.mmk_data().data()["MMK_SOURCES"] {
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
        formatted_string.push_str(&self.print_required_dependencies_libraries()?);
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        Ok(formatted_string)
    }


    fn print_dependencies(&self) -> Result<String, MyMakeError> {
        let borrowed_dependency = self.get_dependency()?.borrow();
        let mut formatted_string = self.print_include_dependency_top()?;
        formatted_string.push_str(&borrowed_dependency.mmk_data().get_include_directories().unwrap());
        if borrowed_dependency.mmk_data().has_system_include() {
            formatted_string.push_str(" ");
            formatted_string.push_str(&borrowed_dependency.mmk_data().to_string("MMK_SYS_INCLUDE"));
        }

        Ok(formatted_string)
    }


    fn print_include_dependency_top(&self) -> Result<String, MyMakeError> {
        let include_line = format!("-I{} ", utility::get_project_top_directory(self.get_dependency()?.borrow().path()).to_str().unwrap());
        Ok(include_line)
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
}


impl Generator for MakefileGenerator
{
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), MyMakeError> {
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
            if !required_dependency.borrow().is_makefile_made() {
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

    fn generate_makefile(self: &mut Self) -> Result<(), MyMakeError> {
        self.include_file_generator.generate_makefiles()?;
        self.create_makefile();
        self.generate_header()?;
        self.generate_appending_flags()?;
        if self.get_dependency()?.borrow().mmk_data().has_executables() {
            self.generate_rule_executable()?;
        }
        else {
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
        ", prerequisites = self.print_prerequisites()?
         , package      = self.print_library_name()?
         , sources_to_objects = self.make_object_rule(&self.get_dependency()?.borrow().mmk_data())?
         , include_headers = self.print_header_includes()?);
        
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
        executable         = self.get_dependency()?.borrow().mmk_data().to_string("MMK_EXECUTABLE"),
        prerequisites      = self.print_prerequisites()?,
        dependencies       = self.print_dependencies()?,
        sources_to_objects = self.make_object_rule(&self.get_dependency()?.borrow().mmk_data())?,
        include_headers = self.print_header_includes()?);
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_appending_flags(&mut self) -> Result<(), MyMakeError> {
        let mut data = String::new();
        let borrowed_dependency = self.get_dependency()?.borrow();
        if borrowed_dependency.mmk_data().data().contains_key("MMK_CXXFLAGS_APPEND") {
            data.push_str(&format!("CXXFLAGS += {cxxflags}\n", 
            cxxflags = borrowed_dependency.mmk_data().to_string("MMK_CXXFLAGS_APPEND")).to_owned());
        }

        if borrowed_dependency.mmk_data().data().contains_key("MMK_CPPFLAGS_APPEND") {
            data.push_str(&format!("CPPFLAGS += {cppflags}\n", 
            cppflags = borrowed_dependency.mmk_data().to_string("MMK_CPPFLAGS_APPEND")).to_owned());
        }

        if !data.is_empty() {
            match self.filename.as_ref().unwrap().write(data.as_bytes()) {
                Ok(_) => (),
                Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
            };
        }
        Ok(())
    }


    fn use_std(&mut self, version: &str) -> Result<(), MyMakeError> {
        self.include_file_generator.add_cpp_version(version)
    }


    fn debug(&mut self) {
        self.debug = true;
        self.use_subdir(std::path::PathBuf::from("debug")).unwrap();
    }


    fn release(&mut self) {
        if !self.debug {
            self.use_subdir(std::path::PathBuf::from("release")).unwrap();
        }
    }


    fn print_ok(self: &Self) -> () {
        print!(".");
    }


    fn set_dependency(&mut self, dependency: &DependencyNode) {
        self.dependency = Some(dependency.clone());
    }


    fn get_dependency(&self) -> Result<&DependencyNode, MyMakeError> {
        if let Some(dep) = &self.dependency {
            return Ok(dep);
        }
        return Err(MyMakeError::from_str("Call on get_dependency when dependency is not set. Call on set_dependency must be done prior!"));
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
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        assert!(Generator::generate_makefile(&mut gen).is_ok());
        Ok(())
    }


    #[test]
    fn print_debug_test() -> std::io::Result<()> {
        let path = std::path::PathBuf::from("some_path");
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&path.join("run.mmk"))));
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        let dir_first_dep = TempDir::new("example_dep")?;
        let dir_second_dep = TempDir::new("example_new_dep")?;
        let output_dir = TempDir::new("build")?;
        let include_dir = dir.path().join("include");
        utility::create_dir(&include_dir).unwrap();
        utility::create_dir(dir_first_dep.path().join("include")).unwrap();
        utility::create_dir(dir_second_dep.path().join("include")).unwrap();
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().add_library_name();
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_REQUIRE".to_string(), vec![dir_first_dep.path().to_str().unwrap().to_string(),
                                                                                               dir_second_dep.path().to_str().unwrap().to_string()]);

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
        \n\
        {directory}/ofilename.o: \\\n\
        \t{dep_directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
        \n\
        sinclude {directory}/filename.d\n\
        sinclude {directory}/ofilename.d\n\
        \n", 
        directory = output_dir.path().to_str().unwrap(),
        dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap(),
        dir_dep_str = dir_first_dep.path().to_str().unwrap().to_string(),
        dir_second_dep_str = dir_second_dep.path().to_str().unwrap().to_string()),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_executable_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let dir_first_dep = TempDir::new("example_dep")?;
        let dir_second_dep = TempDir::new("example_new_dep")?;
        let output_dir = TempDir::new("build")?;
        let include_dir = dir.path().join("include");
        utility::create_dir(&include_dir).unwrap();
        utility::create_dir(dir_first_dep.path().join("include")).unwrap();
        utility::create_dir(dir_second_dep.path().join("include")).unwrap();

        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_EXECUTABLE".to_string(), vec!["x".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_REQUIRE".to_string(), vec![dir_first_dep.path().to_str().unwrap().to_string(),
                                                                                               dir_second_dep.path().to_str().unwrap().to_string()]);
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $^ -o $@)\n\
        \n\
        {directory}/filename.o: \\\n\
        \t{dep_directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
        \n\
        {directory}/ofilename.o: \\\n\
        \t{dep_directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
        \n\
        sinclude {directory}/filename.d\n\
        sinclude {directory}/ofilename.d\n\
        \n",
        directory = output_dir.path().to_str().unwrap(),
        dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap(),
        dir_dep_str = dir_first_dep.path().to_str().unwrap().to_string(),
        dir_second_dep_str = dir_second_dep.path().to_str().unwrap().to_string()),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test_cxxflags() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
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
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let actual = gen.print_header_includes();
        let expected = format!("sinclude {directory}/filename.d\n\
                                       sinclude {directory}/ofilename.d\n",
                                       directory = output_dir.path().to_str().unwrap());
        assert!(actual.is_ok());                                       
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_dependencies_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dir_first_dep = TempDir::new("example_dep")?;
        let dir_second_dep = TempDir::new("example_second_dep")?;
        
        let dep_include_dir = dir_first_dep.path().join("include");
        let second_dep_include_dir = dir_second_dep.path().join("include");
        utility::create_dir(&dep_include_dir).unwrap();
        utility::create_dir(&second_dep_include_dir).unwrap();
        
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_REQUIRE".to_string(), vec![dir_first_dep.path().to_str().unwrap().to_string(),
                                                                                               dir_second_dep.path().to_str().unwrap().to_string()]);

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let expected = format!("-I{} -I{} -I{}", dir.path().to_str().unwrap() , dep_include_dir.to_str().unwrap(), second_dep_include_dir.to_str().unwrap());
        let actual = gen.print_dependencies();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_dependencies_with_sys_include_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dir_first_dep = TempDir::new("example_dep")?;
        let dir_second_dep = TempDir::new("example_second_dep")?;

        let dep_include_dir = dir_first_dep.path().join("include");
        let second_dep_include_dir = dir_second_dep.path().join("include");
        utility::create_dir(&dep_include_dir).unwrap();
        utility::create_dir(&second_dep_include_dir).unwrap();
        
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_REQUIRE".to_string(), vec![dir_first_dep.path().to_str().unwrap().to_string()]);
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SYS_INCLUDE".to_string(), vec![dir_second_dep.path().to_str().unwrap().to_string()]);

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let expected = format!("-I{} -I{} -isystem {}", dir.path().to_str().unwrap(), dep_include_dir.to_str().unwrap(), dir_second_dep.path().to_str().unwrap());
        let actual = gen.print_dependencies();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_dependencies_with_only_sys_include_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dir_dep = TempDir::new("example_dep")?;
        
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_SYS_INCLUDE".to_string(), vec![dir_dep.path().to_str().unwrap().to_string()]);

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let expected = format!("-I{}  -isystem {}", dir.path().to_str().unwrap(), dir_dep.path().to_str().unwrap());
        let actual = gen.print_dependencies();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_required_dependencies_libraries_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dir_dep = TempDir::new("example_dep")?;

        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        let dependency_dep = Rc::new(RefCell::new(Dependency::from(&dir_dep.path().join("lib.mmk"))));
        dependency_dep.borrow_mut().mmk_data_mut().data_mut().insert("MMK_LIBRARY_LABEL".to_string(), vec!["myDependency".to_string()]);
        dependency_dep.borrow_mut().add_library_name();
        dependency.borrow_mut().add_dependency(Rc::clone(&dependency_dep));

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let expected = format!("\t{directory}/libs/{dep_directory}/release/{library_name} \\\n",
                                     directory = output_dir.path().to_str().unwrap(),
                                     dep_directory = dependency_dep.borrow().get_project_name().to_str().unwrap(),
                                     library_name = dependency_dep.borrow().library_file_name());

        let actual = gen.print_required_dependencies_libraries();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_required_dependencies_libraries_multiple_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dir_dep = TempDir::new("example_dep")?;

        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
        let dependency_dep = Rc::new(RefCell::new(Dependency::from(&dir_dep.path().join("lib.mmk"))));
        let second_dependency_dep = Rc::new(RefCell::new(Dependency::from(&dir_dep.path().join("lib.mmk"))));
        dependency_dep.borrow_mut().mmk_data_mut().data_mut().insert("MMK_LIBRARY_LABEL".to_string(), vec!["myDependency".to_string()]);
        dependency_dep.borrow_mut().add_library_name();
        second_dependency_dep.borrow_mut().mmk_data_mut().data_mut().insert("MMK_LIBRARY_LABEL".to_string(), vec!["mySecondDependency".to_string()]);
        second_dependency_dep.borrow_mut().add_library_name();
        dependency.borrow_mut().add_dependency(Rc::clone(&dependency_dep));
        dependency.borrow_mut().add_dependency(Rc::clone(&second_dependency_dep));

        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);
        let expected = format!("\t{directory}/libs/{dep_directory}/release/{library_name} \\\n\
                                       \t{directory}/libs/{second_dep_directory}/release/{second_library_name} \\\n",
                                     directory = output_dir.path().to_str().unwrap(),
                                     dep_directory = dependency_dep.borrow().get_project_name().to_str().unwrap(),
                                     second_dep_directory = second_dependency_dep.borrow().get_project_name().to_str().unwrap(),
                                     library_name = dependency_dep.borrow().library_file_name(),
                                     second_library_name = second_dependency_dep.borrow().library_file_name());

        let actual = gen.print_required_dependencies_libraries();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_library_name_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);

        let expected = format!("{directory}/{library_file_name}",
                                      directory = output_dir.path().to_str().unwrap(),
                                      library_file_name = dependency.borrow().library_file_name());
        let actual = gen.print_library_name();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }


    #[test]
    fn print_library_name_with_label_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = TempDir::new("build")?;
        let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
        dependency.borrow_mut().mmk_data_mut().data_mut().insert("MMK_LIBRARY_LABEL".to_string(), vec!["myDependency".to_string()]);
        dependency.borrow_mut().add_library_name();
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        gen.set_dependency(&dependency);

        let expected = format!("{directory}/libmyDependency.a",
                                      directory = output_dir.path().to_str().unwrap());
        let actual = gen.print_library_name();
        assert!(actual.is_ok());
        assert_eq!(expected, actual.unwrap());
        Ok(())
    }

    #[test]
    fn replace_generator_test() -> std::io::Result<()> {
        let dir_dep = TempDir::new("example_dep")?;
        let output_dir = TempDir::new("build")?;
        let replacement_output_dir = dir_dep.path().join("build");

        let dependency = Rc::new(RefCell::new(Dependency::from(&dir_dep.path().join("lib.mmk"))));
        let mut gen = MakefileGenerator::new(output_dir.path().to_path_buf());
        
        gen.replace_generator(&dependency, replacement_output_dir.clone());
        assert_eq!(dependency, gen.dependency.unwrap());
        assert_eq!(replacement_output_dir, gen.output_directory);

        Ok(())
    }
    
}
