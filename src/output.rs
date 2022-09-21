use colored::Colorize;

// TODO: Task - relatert output? Mulighet for å starte en task og avslutte den med en eller annen form
// for statusmelding fra Output.

// Eks: Task - struct som gjør en oppgave og ved oppgavets slutt returnerer en statusmelding for det
// som ved generering av makefiler.
// ```
//  output.task("Generating makefiles", || { builder.generate_makefiles() })
//
// Skal output - rammeverket ha forhold til logging?

const YAMBS_PREFIX: &str = "yambs";

pub struct Output {
    inner: InnerOutput,
}

impl Output {
    pub fn new() -> Self {
        Self {
            inner: InnerOutput::new(),
        }
    }

    #[allow(unused)]
    pub fn status(&self, output: &str) {
        self.inner.print(output, OutputType::Status);
        log::info!("{}", output);
    }

    #[allow(unused)]
    pub fn warning(&self, output: &str) {
        self.inner.print(output, OutputType::Warning);
        log::warn!("{}", output);
    }

    #[allow(unused)]
    pub fn error(&self, output: &str) {
        self.inner.print(output, OutputType::Error);
        log::error!("{}", output);
    }
}

struct InnerOutput {
    prefix: String,
}

impl InnerOutput {
    pub fn new() -> Self {
        Self {
            prefix: YAMBS_PREFIX.to_string(),
        }
    }

    fn print(&self, output: &str, output_type: OutputType) {
        let prepared_output = self.add_prefix(output);
        let color = output_type.as_color();

        match output_type {
            OutputType::Status | OutputType::Warning => {
                println!("{}", prepared_output.color(color))
            }
            OutputType::Error => eprintln!("{}", prepared_output.color(color)),
        };
    }

    fn add_prefix(&self, output: &str) -> String {
        format!("{}: {}", self.prefix, output)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_output_type_is_white() {
        assert_eq!(OutputType::Status.as_color(), colored::Color::White);
    }

    #[test]
    fn warning_output_type_is_yellow() {
        assert_eq!(OutputType::Warning.as_color(), colored::Color::Yellow);
    }

    #[test]
    fn error_output_type_is_red() {
        assert_eq!(OutputType::Error.as_color(), colored::Color::Red);
    }
}
