use crate::errors::LoggerError;

pub const YAMBS_LOG_FILE: &str = "yambs_log.txt";

pub struct Logger {
    _handle: log4rs::Handle,
    path: std::path::PathBuf,
}

impl Logger {
    pub fn init(
        log_directory: &std::path::Path,
        log_level: log::LevelFilter,
    ) -> Result<Logger, LoggerError> {
        let path = log_directory.join(YAMBS_LOG_FILE);
        let logfile = log4rs::append::file::FileAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                r"[{d(%Y-%m-%d %H:%M:%S)}] [{l}] [\({t}\)]  - {m}{n}",
            )))
            .append(false)
            .build(&path)
            .map_err(LoggerError::FailedToCreateFileAppender)?;

        let config = log4rs::Config::builder()
            .appender(log4rs::config::Appender::builder().build("logfile", Box::new(logfile)))
            .build(
                log4rs::config::Root::builder()
                    .appender("logfile")
                    .build(log_level),
            )
            .map_err(LoggerError::FailedToCreateConfig)?;
        let _handle = log4rs::init_config(config)?;

        Ok(Self { _handle, path })
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.path.to_owned()
    }
}
