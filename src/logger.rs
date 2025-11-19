use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

pub fn init_logger(log_level: &str) {
    let level_filter = match log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({d(%Y-%m-%d %H:%M:%S)} [{l}] {M}-{f}:{L})} -> {m}{n}")))
        .build();

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({d(%Y-%m-%d %H:%M:%S)} [{l}] {M}-{f}:{L})} -> {m}{n}")))
        .build("logs/my_cache.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(
            Logger::builder()
                .appender("stdout")
                .appender("logfile")
                .additive(false)
                .build("my_cache", level_filter),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(level_filter),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
