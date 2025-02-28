use fern::FormatCallback;
use fern::colors::{Color, ColoredLevelConfig};
use log::{LevelFilter, Record};
use std::fmt::Arguments;
use std::path::Path;
pub async fn init() -> Result<(), fern::InitError> {
    #[cfg(target_os = "linux")]
    let path = Path::new("/var/log/ehs/exchanger/");
    #[cfg(target_os = "macos")]
    let path = Path::new("/Users/wilson/Downloads/logs/ehs/exchanger/");

    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::BrightGreen)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    let formatter = move |out: FormatCallback, message: &Arguments, record: &Record| {
        out.finish(format_args!(
            "{} {}@{} {}",
            chrono::Local::now().format("%Y/%m/%dT%H:%M:%S%.3f"),
            colors.color(record.level()),
            record.target(),
            message
        ));
    };

    let info = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::DateBased::new(path.join("info"), "@%Y-%m-%d.log"));

    let error = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Error)
        .chain(std::io::stdout())
        .chain(fern::DateBased::new(path.join("error"), "@%Y-%m-%d.log"));

    let warn = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Warn)
        .chain(std::io::stdout())
        .chain(fern::DateBased::new(path.join("warning"), "@%Y-%m-%d.log"));

    fern::Dispatch::new()
        .chain(info)
        .chain(error)
        .chain(warn)
        .apply()?;
    Ok(())
}
