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
    filter_string_regex(input, pattern_ar.unwrap())
}

#[allow(dead_code)]
pub fn is_warning_message(input: &str) -> bool{
    let warning_pattern_gcc = Regex::new(r".*\[-W.*\]$").unwrap();
    warning_pattern_gcc.is_match(input)
}

pub fn println_colored(input: &String) {
    input.lines().for_each(|line| { if is_warning_message(&line) {
                                            println!("{}", format!("{}", line).yellow());
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
    fn is_warning_message_test() {
        let input = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wunused-variable]";
        let input_narrowing = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wnarrowing]";
        let input_uninitialized = "/sadfasdfsaf/fasdfdf sadfasf fsadf [-Wuninitialized]";
        assert!(is_warning_message(&input) == true);
        assert!(is_warning_message(&input_narrowing) == true);
        assert!(is_warning_message(&input_uninitialized) == true);
    }
}