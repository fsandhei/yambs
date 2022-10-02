use std::string::String;

use regex::Regex;

use crate::output;

fn filter_string_regex(input: &str, pattern: Regex) -> String {
    pattern.replace(input, "").trim_start().to_string()
}

pub fn filter_string(input: &str) -> String {
    let pattern_ar = Regex::new(r"^ar.*\n+").unwrap();
    let pattern_ar_second = Regex::new(r"\nar:.*").unwrap();
    let pattern_ar_open = Regex::new(r".*ar:.*").unwrap();

    filter_string_regex(input, pattern_ar);
    filter_string_regex(input, pattern_ar_second);
    filter_string_regex(input, pattern_ar_open)
}

fn is_warning_message(input: &str) -> bool {
    let warning_pattern_gcc = Regex::new(r".*\[-W.*\]$").unwrap();
    warning_pattern_gcc.is_match(input)
}

fn is_error_message(input: &str) -> bool {
    let error_pattern_gcc = Regex::new(r".* error:.*").unwrap();
    error_pattern_gcc.is_match(input)
}

pub fn println_colored(input: &str, output: &output::Output) {
    input.lines().for_each(|line| {
        if is_warning_message(line) {
            output.warning_without_prefix(&format!("{}", line));
        } else if is_error_message(line) {
            output.error_without_prefix(&format!("{}", line));
        } else {
            output.status_without_prefix(&format!("{}", line));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_string_remove_ar_test() {
        let input = String::from(
            "ar: asdfsadfsadf \n
                                        /sdadfsadfasfsf/",
        );
        let expected_output = "/sdadfsadfasfsf/";
        let pattern = Regex::new(r"^ar.*\n+");

        assert_eq!(
            expected_output,
            filter_string_regex(&input, pattern.unwrap())
        );
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
    fn is_warning_message_false_test() {
        let input = "/sadfasdfsaf/fasdfdf sadfasf fsadf this is not a warning!";
        assert!(!is_warning_message(input));
    }

    #[test]
    fn is_error_message_test() {
        let input = "\
        /home/user/Documents/Tests/AStarPathFinder/PlanGenerator/test/PlanGeneratorTest.cpp:32:13: error: ‘dfasdf’
        was not declared in this scope";
        assert!(is_error_message(input));
    }

    #[test]
    fn is_error_message_false_test() {
        let input = "\
        This is not an error!";
        assert!(!is_error_message(input));
    }
}
