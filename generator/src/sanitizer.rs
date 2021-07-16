pub trait Sanitizer {
    fn set_sanitizers(&mut self, sanitizers: Vec<&str>);
}