use std::sync::{atomic::AtomicBool, Arc, Mutex};

use crates_io_api::CratesQuery;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

/// Represents the parameters needed for fetching crates asynchronously.
pub struct SearchParameters {
    pub search: String,
    pub page: u64,
    pub page_size: u64,
    pub crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
    pub loading_status: Arc<AtomicBool>,
    pub sort: crates_io_api::Sort,
    pub tx: UnboundedSender<Action>,
}

/// Performs the actual search, and sends the result back through the
/// sender.
pub async fn request_crates(params: &SearchParameters) -> Result<(), String> {
    // Fetch crates using the created client with the error handling in one place.
    let client = create_client()?;
    let query = create_query(&params);
    let crates = fetch_crates(client, query).await?;
    update_state_with_fetched_crates(crates, params);
    Ok(())
}

/// Helper function to create client and fetch crates, wrapping both actions
/// into a result pattern.
fn create_client() -> Result<crates_io_api::AsyncClient, String> {
    // Attempt to create the API client
    crates_io_api::AsyncClient::new(
        "crates-tui (crates-tui@kdheepak.com)",
        std::time::Duration::from_millis(1000),
    )
    .map_err(|err| format!("API Client Error: {err:#?}"))
}

fn create_query(params: &SearchParameters) -> CratesQuery {
    // Form the query and fetch the crates, passing along any errors.
    crates_io_api::CratesQueryBuilder::default()
        .search(&params.search)
        .page(params.page)
        .page_size(params.page_size)
        .sort(params.sort.clone())
        .build()
}

async fn fetch_crates(
    client: crates_io_api::AsyncClient,
    query: crates_io_api::CratesQuery,
) -> Result<Vec<crates_io_api::Crate>, String> {
    let page_result = client
        .crates(query)
        .await
        .map_err(|err| format!("API Client Error: {err:#?}"))?;
    let mut crates = page_result.crates;
    crates.sort_by(|a, b| b.downloads.cmp(&a.downloads));

    Ok(crates)
}

/// Handles the result after fetching crates and sending corresponding
/// actions.
fn update_state_with_fetched_crates(crates: Vec<crates_io_api::Crate>, params: &SearchParameters) {
    // Lock and update the shared state container
    let mut app_crates = params.crates.lock().unwrap();
    app_crates.clear();
    app_crates.extend(crates);

    // After a successful fetch, send relevant actions based on the result
    if app_crates.is_empty() {
        let _ = params.tx.send(Action::ShowErrorPopup(format!(
            "Could not find any crates with query `{}`.",
            params.search
        )));
    } else {
        let _ = params
            .tx
            .send(Action::StoreTotalNumberOfCrates(app_crates.len() as u64));
        let _ = params.tx.send(Action::Tick);
        let _ = params.tx.send(Action::ScrollDown);
    }
}

// Performs the async fetch of crate details.
pub async fn request_crate_details(
    crate_name: String,
    crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
) -> Result<(), String> {
    let client = create_client()?;

    let crate_data = client
        .get_crate(&crate_name)
        .await
        .map_err(|err| format!("Error fetching crate details: {err:#?}"))?;

    *crate_info.lock().unwrap() = Some(crate_data.crate_data);
    Ok(())
}
