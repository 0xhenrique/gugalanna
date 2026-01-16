//! Console API implementation
//!
//! Provides console.log, console.warn, console.error, etc.

use rquickjs::{Ctx, Function, Object, Result};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Log level for console messages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
    Debug,
}

/// A captured console message
#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: Instant,
}

/// Shared console message storage
pub type ConsoleMessages = Arc<Mutex<Vec<ConsoleMessage>>>;

/// Create a new console message storage
pub fn new_console_messages() -> ConsoleMessages {
    Arc::new(Mutex::new(Vec::new()))
}

/// Register the console object in the global scope with message storage
pub fn register_console(ctx: &Ctx<'_>, messages: ConsoleMessages) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    // console.log - simplified to accept string arguments
    let log_messages = messages.clone();
    console.set(
        "log",
        Function::new(ctx.clone(), move |msg: String| {
            log::info!("[JS] {}", msg);
            println!("[console.log] {}", msg);
            if let Ok(mut msgs) = log_messages.lock() {
                msgs.push(ConsoleMessage {
                    level: LogLevel::Log,
                    message: msg,
                    timestamp: Instant::now(),
                });
            }
        })?,
    )?;

    // console.warn
    let warn_messages = messages.clone();
    console.set(
        "warn",
        Function::new(ctx.clone(), move |msg: String| {
            log::warn!("[JS] {}", msg);
            println!("[console.warn] {}", msg);
            if let Ok(mut msgs) = warn_messages.lock() {
                msgs.push(ConsoleMessage {
                    level: LogLevel::Warn,
                    message: msg,
                    timestamp: Instant::now(),
                });
            }
        })?,
    )?;

    // console.error
    let error_messages = messages.clone();
    console.set(
        "error",
        Function::new(ctx.clone(), move |msg: String| {
            log::error!("[JS] {}", msg);
            eprintln!("[console.error] {}", msg);
            if let Ok(mut msgs) = error_messages.lock() {
                msgs.push(ConsoleMessage {
                    level: LogLevel::Error,
                    message: msg,
                    timestamp: Instant::now(),
                });
            }
        })?,
    )?;

    // console.info (alias for log)
    let info_messages = messages.clone();
    console.set(
        "info",
        Function::new(ctx.clone(), move |msg: String| {
            log::info!("[JS] {}", msg);
            println!("[console.info] {}", msg);
            if let Ok(mut msgs) = info_messages.lock() {
                msgs.push(ConsoleMessage {
                    level: LogLevel::Info,
                    message: msg,
                    timestamp: Instant::now(),
                });
            }
        })?,
    )?;

    // console.debug
    let debug_messages = messages.clone();
    console.set(
        "debug",
        Function::new(ctx.clone(), move |msg: String| {
            log::debug!("[JS] {}", msg);
            println!("[console.debug] {}", msg);
            if let Ok(mut msgs) = debug_messages.lock() {
                msgs.push(ConsoleMessage {
                    level: LogLevel::Debug,
                    message: msg,
                    timestamp: Instant::now(),
                });
            }
        })?,
    )?;

    globals.set("console", console)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::Runtime;

    #[test]
    fn test_console_log() {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        let messages = new_console_messages();
        ctx.with(|ctx| {
            register_console(&ctx, messages.clone()).unwrap();
            let _: () = ctx.eval("console.log('Hello World')").unwrap();
        });

        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].level, LogLevel::Log);
        assert_eq!(msgs[0].message, "Hello World");
    }

    #[test]
    fn test_console_error() {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        let messages = new_console_messages();
        ctx.with(|ctx| {
            register_console(&ctx, messages.clone()).unwrap();
            let _: () = ctx.eval("console.error('Error message')").unwrap();
        });

        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].level, LogLevel::Error);
        assert_eq!(msgs[0].message, "Error message");
    }

    #[test]
    fn test_all_log_levels() {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        let messages = new_console_messages();
        ctx.with(|ctx| {
            register_console(&ctx, messages.clone()).unwrap();
            let _: () = ctx.eval(r#"
                console.log('log');
                console.info('info');
                console.warn('warn');
                console.error('error');
                console.debug('debug');
            "#).unwrap();
        });

        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0].level, LogLevel::Log);
        assert_eq!(msgs[1].level, LogLevel::Info);
        assert_eq!(msgs[2].level, LogLevel::Warn);
        assert_eq!(msgs[3].level, LogLevel::Error);
        assert_eq!(msgs[4].level, LogLevel::Debug);
    }
}
