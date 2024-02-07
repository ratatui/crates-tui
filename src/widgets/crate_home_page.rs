use std::path::PathBuf;

use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::*;

#[derive(Debug, Clone)]
struct CrateHomePage<'a> {
    full_crate: &'a crates_io_api::FullCrate,
}

impl<'a> CrateHomePage<'a> {
    pub fn new() -> Result<Self> {
        let ratatui_full_crate = include_str!("./../../.data/ratatui-full-crate.toml");
        let full_crate = &toml::from_str(ratatui_full_crate)?;
        Ok(CrateHomePage { full_crate })
    }
}

struct CrateHomePageWidget {}

impl StatefulWidget for CrateHomePageWidget {
    type State = CrateHomePage;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;
        let [header, main] = Layout::vertical([Length(5), Fill(0)]).areas(area);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    /*
    #[tokio::test]
    async fn load_ratatui() -> Result<(), String> {
        use std::sync::{Arc, Mutex};
        use crate::crates_io_api_helper;
        let full_crate_info: Arc<Mutex<Option<crates_io_api::FullCrate>>> = Default::default();
        println!("Requesting...");
        let _ci = full_crate_info.clone();
        crates_io_api_helper::request_full_crate_details("ratatui", _ci).await?;
        if let Some(ref full_crate) = full_crate_info.lock().unwrap().clone() {
            println!("{}", toml::to_string_pretty(full_crate).ok().unwrap());
        }

        Ok(())
    }
    */
}
