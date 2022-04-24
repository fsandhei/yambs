use colored::Colorize;

// TODO: Task - relatert output? Mulighet for å starte en task og avslutte den med en eller annen form
// for statusmelding fra Output.

// Eks: Task - struct som gjør en oppgave og ved oppgavets slutt returnerer en statusmelding for det
// som ved generering av makefiler.
// ```
//  output.task("Generating makefiles", || { builder.generate_makefiles() })
//
// Skal output - rammeverket ha forhold til logging?

const RSMAKE_PREFIX: &str = "rsmake";

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
    }

    #[allow(unused)]
    pub fn warning(&self, output: &str) {
        self.inner.print(output, OutputType::Warning);
    }

    #[allow(unused)]
    pub fn error(&self, output: &str) {
        self.inner.print(output, OutputType::Error);
    }
}

struct InnerOutput {
    prefix: String,
}

impl InnerOutput {
    pub fn new() -> Self {
        Self {
            prefix: RSMAKE_PREFIX.to_string(),
        }
    }

    fn print(&self, output: &str, output_type: OutputType) {
        let prepared_output = self.prepare_output(output, &output_type);

        match output_type {
            OutputType::Status | OutputType::Warning => println!("{}", prepared_output),
            OutputType::Error => eprintln!("{}", prepared_output),
        };
    }

    fn prepare_output(&self, output: &str, output_type: &OutputType) -> String {
        let color = match output_type {
            OutputType::Status => colored::Color::White,
            OutputType::Warning => colored::Color::Yellow,
            OutputType::Error => colored::Color::Red,
        };
        self.add_prefix(&output.color(color))
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

// enum Cli {
//     Normal,
//     Warning,
//     Error,
// }

// enum Log {
//     Trace,
//     Debug,
//     Info,
//     Warning,
//     Error,
// }
