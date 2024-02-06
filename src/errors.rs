use std::panic;

use color_eyre::{
    config::{EyreHook, HookBuilder, PanicHook},
    eyre::{self, Result},
};
use tracing::error;

use crate::tui;

pub fn install_hooks() -> Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();

    #[cfg(debug_assertions)]
    install_better_panic();
    #[cfg(not(debug_assertions))]
    install_human_panic();

    install_color_eyre_panic_hook(panic_hook);
    install_eyre_hook(eyre_hook)?;

    Ok(())
}

#[allow(dead_code)]
fn install_better_panic() {
    better_panic::Settings::auto()
        .most_recent_first(false)
        .verbosity(better_panic::Verbosity::Full)
        .install()
}

#[allow(dead_code)]
fn install_human_panic() {
    human_panic::setup_panic!(Metadata {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: env!("CARGO_PKG_AUTHORS").replace(':', ", ").into(),
        homepage: env!("CARGO_PKG_HOMEPAGE").into(),
    });
}

fn install_color_eyre_panic_hook(panic_hook: PanicHook) {
    // convert from a `color_eyre::config::PanicHook`` to a `Box<dyn
    // Fn(&PanicInfo<'_>`
    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Err(err) = tui::restore_backend() {
            error!("Unable to restore terminal: {err:?}");
        }

        // not sure about this
        // let msg = format!("{}", panic_hook.panic_report(panic_info));
        // error!("Error: {}", strip_ansi_escapes::strip_str(msg));
        panic_hook(panic_info);
    }));
}

fn install_eyre_hook(eyre_hook: EyreHook) -> color_eyre::Result<()> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        tui::restore_backend().unwrap();
        eyre_hook(error)
    }))?;
    Ok(())
}
