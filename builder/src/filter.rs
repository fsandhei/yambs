use regex::Regex;
use std::string::String;
use colored::Colorize;


/*
    * Lag filter som fjerner AR - tekst output fra stderr
    * 
*/
#[allow(dead_code)]
fn filter_string_regex(input: &String, pattern: Regex) -> String {
    pattern.replace(input, "").trim_start().to_string()
}

pub fn filter_string(input: &String) -> String {
    let pattern_ar = Regex::new(r"^ar.*\n+");
    let pattern_ar_second = Regex::new(r"\nar:.*");
    let pattern_ar_open = Regex::new(r".*ar:.*");
    filter_string_regex(input, pattern_ar.unwrap());
    filter_string_regex(input, pattern_ar_second.unwrap());
    filter_string_regex(input, pattern_ar_open.unwrap())
}

#[allow(dead_code)]
fn is_warning_message(input: &str) -> bool{
    let warning_pattern_gcc = Regex::new(r".*\[-W.*\]$").unwrap();
    warning_pattern_gcc.is_match(input)
}

#[allow(dead_code)]
fn is_error_message(input: &str) -> bool{
    let error_pattern_gcc = Regex::new(r".* error:.*").unwrap();
    error_pattern_gcc.is_match(input)
}

pub fn println_colored(input: &String) {
    input.lines().for_each(|line| { if is_warning_message(&line) {
                                            println!("{}", format!("{}", line).yellow());
                                       }
                                        else if is_error_message(&line) {
                                            println!("{}", format!("{}", line).red())
                                        }
                                        else {
                                            println!("{}", line);
                                        }                                        
                                    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_string_remove_ar_test() {
        let input = String::from("ar: asdfsadfsadf \n
                                        /sdadfsadfasfsf/");
        let expected_output = "/sdadfsadfasfsf/";
        let pattern = Regex::new(r"^ar.*\n+");
        
        assert_eq!(expected_output, filter_string_regex(&input, pattern.unwrap()));
    }


    #[test]
    fn filter_string_nothing_to_filter_test() {
        let input = String::from("This is a string with nothing to be filtered.");
        let expected_output = input.clone();
        assert_eq!(expected_output, filter_string(&input));
    }


    #[test]
    fn filter_string_remove_ar_creating_test() {
        let input = String::from("\nar: creating /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/googletest/");
        let expected_output = "";
        
        assert_eq!(expected_output, filter_string(&input));
    }

    #[test]
    fn is_warning_message_test() {
        let input = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wunused-variable]";
        let input_narrowing = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wnarrowing]";
        let input_uninitialized = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wuninitialized]";
        assert!(is_warning_message(&input) == true);
        assert!(is_warning_message(&input_narrowing) == true);
        assert!(is_warning_message(&input_uninitialized) == true);
    }

    #[test]
    fn is_error_message_test() {
        let input = "\
        /home/user/Documents/Tests/AStarPathFinder/PlanGenerator/test/PlanGeneratorTest.cpp:32:13: error: ‘dfasdf’
        was not declared in this scope";
        assert!(is_error_message(&input) == true);
    }
}