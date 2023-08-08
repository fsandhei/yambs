use std::string::String;

use regex::Regex;

use crate::output;

pub fn filter_string(input: &str) -> String {
    let pattern_ar = Regex::new(r"^ar.*\n+").unwrap();
    let pattern_ar_second = Regex::new(r"\nar:.*").unwrap();
    let pattern_ar_open = Regex::new(r".*ar:.*").unwrap();

    input
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| !pattern_ar.is_match(line))
        .filter(|line| !pattern_ar_second.is_match(line))
        .filter(|line| !pattern_ar_open.is_match(line))
        .collect::<String>()
}

pub fn print_error_colored(input: &str, output: &output::Output) {
    output.error_without_prefix(input);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_string_remove_ar_test() {
        let input = String::from(
            "ar: asdfsadfsadf \n\
            /sdadfsadfasfsf/",
        );
        let expected_output = "/sdadfsadfasfsf/";

        assert_eq!(expected_output, filter_string(&input));
    }

    #[test]
    fn filter_string_nothing_to_filter_test() {
        let input = String::from("This is a string with nothing to be filtered.");
        let expected_output = input.clone();
        assert_eq!(expected_output, filter_string(&input));
    }

    #[test]
    fn fno_newlineilter_string_remove_ar_creating_test_multiple_lines() {
        let input = String::from("ar: creating visitorlibrary.a");
        let expected_output = "";

        assert_eq!(expected_output, filter_string(&input));
    }

    #[test]
    fn filter_string_remove_ar_creating_test() {
        let input = String::from("\nar: creating /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/googletest/");
        let expected_output = "";

        assert_eq!(expected_output, filter_string(&input));
    }
}
