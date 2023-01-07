use colored::Colorize;

pub mod filter;

const YAMBS_PREFIX: &str = "yambs";

#[derive(Debug, Clone)]
pub struct Output {
    inner: std::sync::Arc<InnerOutput>,
}

impl Output {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(InnerOutput::new()),
        }
    }

    pub fn status(&self, text: &str) {
        self.inner
            .print(text, OutputType::Status, PrefixPolicy::WithPrefix);
        log::info!("{}", text);
    }

    pub fn status_without_prefix(&self, text: &str) {
        self.inner
            .print(text, OutputType::Status, PrefixPolicy::NoPrefix);
        log::info!("{}", text);
    }

    pub fn warning(&self, text: &str) {
        self.inner
            .print(text, OutputType::Warning, PrefixPolicy::WithPrefix);
        log::warn!("{}", text);
    }

    pub fn warning_without_prefix(&self, text: &str) {
        self.inner
            .print(text, OutputType::Warning, PrefixPolicy::NoPrefix);
        log::warn!("{}", text);
    }

    pub fn error(&self, text: &str) {
        self.inner
            .print(text, OutputType::Error, PrefixPolicy::WithPrefix);
        log::error!("{}", text);
    }

    pub fn error_without_prefix(&self, text: &str) {
        self.inner
            .print(text, OutputType::Error, PrefixPolicy::NoPrefix);
        log::error!("{}", text);
    }
}

#[derive(Debug)]
struct InnerOutput {
    prefix: String,
}

impl InnerOutput {
    pub fn new() -> Self {
        Self {
            prefix: YAMBS_PREFIX.to_string(),
        }
    }

    fn print(&self, text: &str, text_type: OutputType, prefix_policy: PrefixPolicy) {
        let prepared_text = self.add_prefix(text, prefix_policy);
        let color = text_type.as_color();

        match text_type {
            OutputType::Status | OutputType::Warning => {
                println!("{}", prepared_text.color(color))
            }
            OutputType::Error => eprintln!("{}", prepared_text.color(color)),
        };
    }

    fn add_prefix(&self, text: &str, prefix_policy: PrefixPolicy) -> String {
        match prefix_policy {
            PrefixPolicy::WithPrefix => format!("{}: {}", self.prefix, text),
            PrefixPolicy::NoPrefix => text.to_string(),
        }
    }
}

lazy_static::lazy_static! {
    static ref DEFAULT_PROGRESS_BAR_STYLE: indicatif::ProgressStyle =
        indicatif::ProgressStyle::with_template("[{bar:.cyan/blue}] [{elapsed_precise}] {msg}")
                .unwrap()
                .progress_chars("=>-");
}

pub struct ProgressBar {
    pub bar: indicatif::ProgressBar,
}

impl ProgressBar {
    pub fn new(len: u64) -> Self {
        let pb = indicatif::ProgressBar::new(len);
        pb.set_style(DEFAULT_PROGRESS_BAR_STYLE.clone());
        Self { bar: pb }
    }

    pub fn finish_with_message(&self, msg: impl AsRef<str>) {
        self.bar.println(msg);
        self.bar.finish_and_clear();
    }

    pub fn fail_with_message(&self, msg: impl Into<std::borrow::Cow<'static, str>>) {
        self.bar.abandon_with_message(msg);
    }
}

enum OutputType {
    Status,
    Warning,
    Error,
}

impl OutputType {
    pub fn as_color(&self) -> colored::Color {
        match self {
            OutputType::Status => colored::Color::White,
            OutputType::Warning => colored::Color::Yellow,
            OutputType::Error => colored::Color::Red,
        }
    }
}

enum PrefixPolicy {
    WithPrefix,
    NoPrefix,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_type_is_white() {
        assert_eq!(OutputType::Status.as_color(), colored::Color::White);
    }

    #[test]
    fn warning_text_type_is_yellow() {
        assert_eq!(OutputType::Warning.as_color(), colored::Color::Yellow);
    }

    #[test]
    fn error_text_type_is_red() {
        assert_eq!(OutputType::Error.as_color(), colored::Color::Red);
    }
}
