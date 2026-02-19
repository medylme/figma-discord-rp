use std::fmt;

use chrono::Local;
use owo_colors::OwoColorize;

use crate::VERSION;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

fn print_colored(level: LogLevel, module: &str, message: &str) {
    let version_str = if cfg!(debug_assertions) {
        "dev".to_string()
    } else {
        format!("v{}", VERSION)
    };

    let level_str = match level {
        LogLevel::Debug => format!("{:5}", level).blue().to_string(),
        LogLevel::Info => format!("{:5}", level).green().to_string(),
        LogLevel::Warn => format!("{:5}", level).yellow().to_string(),
        LogLevel::Error => format!("{:5}", level).red().to_string(),
    };

    let timestamp = Local::now().format("%H:%M:%S%.3f");
    let module_str = format!("[{}]", module).cyan().to_string();

    println!(
        "{} {}  {}  {} {}",
        timestamp.dimmed(),
        version_str.dimmed(),
        level_str,
        module_str,
        message
    );
}

pub fn log(level: LogLevel, module: &str, message: String) {
    print_colored(level, module, &message);
}

#[cfg(debug_assertions)]
pub fn log_debug(module: &str, message: String) {
    log(LogLevel::Debug, module, message);
}

#[cfg(not(debug_assertions))]
pub fn log_debug(_module: &str, _message: String) {}

pub fn log_info(module: &str, message: String) {
    log(LogLevel::Info, module, message);
}

pub fn log_warn(module: &str, message: String) {
    log(LogLevel::Warn, module, message);
}

pub fn log_error(module: &str, message: String) {
    log(LogLevel::Error, module, message);
}

#[macro_export]
macro_rules! log_debug {
    ($module:literal, $($arg:tt)*) => {
        $crate::logging::log_debug($module, format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($module:literal, $($arg:tt)*) => {
        $crate::logging::log_info($module, format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:literal, $($arg:tt)*) => {
        $crate::logging::log_warn($module, format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($module:literal, $($arg:tt)*) => {
        $crate::logging::log_error($module, format!($($arg)*));
    };
}
