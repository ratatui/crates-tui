use color_eyre::eyre::Result;

use cfg_if::cfg_if;

pub fn install_hooks() -> Result<()> {
    cfg_if! {
        if #[cfg(debug_assertions)] {
            install_better_panic();
        } else {
            human_panic::setup_panic!();
        }
    }
    color_eyre::install()
}

#[allow(dead_code)]
fn install_better_panic() {
    better_panic::Settings::auto()
        .most_recent_first(false)
        .verbosity(better_panic::Verbosity::Full)
        .install()
}
