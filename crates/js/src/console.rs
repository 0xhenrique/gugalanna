//! Console API implementation
//!
//! Provides console.log, console.warn, console.error, etc.

use rquickjs::{Ctx, Function, Object, Result};

/// Register the console object in the global scope
pub fn register_console(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    // console.log - simplified to accept string arguments
    console.set(
        "log",
        Function::new(ctx.clone(), |msg: String| {
            log::info!("[JS] {}", msg);
            println!("[console.log] {}", msg);
        })?,
    )?;

    // console.warn
    console.set(
        "warn",
        Function::new(ctx.clone(), |msg: String| {
            log::warn!("[JS] {}", msg);
            println!("[console.warn] {}", msg);
        })?,
    )?;

    // console.error
    console.set(
        "error",
        Function::new(ctx.clone(), |msg: String| {
            log::error!("[JS] {}", msg);
            eprintln!("[console.error] {}", msg);
        })?,
    )?;

    // console.info (alias for log)
    console.set(
        "info",
        Function::new(ctx.clone(), |msg: String| {
            log::info!("[JS] {}", msg);
            println!("[console.info] {}", msg);
        })?,
    )?;

    // console.debug
    console.set(
        "debug",
        Function::new(ctx.clone(), |msg: String| {
            log::debug!("[JS] {}", msg);
            println!("[console.debug] {}", msg);
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

        ctx.with(|ctx| {
            register_console(&ctx).unwrap();
            let _: () = ctx.eval("console.log('Hello World')").unwrap();
        });
    }

    #[test]
    fn test_console_error() {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            register_console(&ctx).unwrap();
            let _: () = ctx.eval("console.error('Error message')").unwrap();
        });
    }
}
