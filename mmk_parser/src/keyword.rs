
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Keyword {
    argument: String,
    option: String,
}

impl Keyword {
    pub fn new() -> Self {
        Keyword { argument: String::new(), option: String::new() }
    }


    pub fn with_argument(mut self, arg: &str) -> Self {
        self.argument = arg.to_string();
        self
    }


    pub fn with_option(mut self, option: &str) -> Self{
        self.option = option.to_string();
        self
    }


    pub fn argument(&self) -> &String {
        &self.argument
    }


    pub fn option(&self) -> &String {
        &self.option
    }
}


impl From<&str> for Keyword {
    fn from(s: &str) -> Self {
        Keyword { argument: String::from(s), option: String::new() }
    }
}


impl From<&String> for Keyword {
    fn from(s: &String) -> Self {
        Keyword { argument: String::from(s), option: String::new() }
    }
}
