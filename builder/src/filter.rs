use regex::Regex;
use std::string::String;


/*
    * Lag filter som fjerner AR - tekst output fra stderr
    * 
*/
#[allow(dead_code)]
fn filter_string_regex(input: &String, pattern: Regex) -> String {
    
    // let pattern = Regex::new(r"^?/(.*)").unwrap();
    pattern.replace(input, "").trim_start().to_string()
}

pub fn filter_string(input: &String) -> String {
    let pattern_ar = Regex::new(r"^ar.*\n+");
    filter_string_regex(input, pattern_ar.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_string_remove_ar() {
        let input = String::from("ar: asdfsadfsadf \n
                                        /sdadfsadfasfsf/");
        let expected_output = "/sdadfsadfasfsf/";
        let pattern = Regex::new(r"^ar.*\n+");
        
        assert_eq!(expected_output, filter_string_regex(&input, pattern.unwrap()));
    }
}