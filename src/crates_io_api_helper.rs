use std::sync::{Arc, Mutex, atomic::AtomicBool};

use crates_io_api::CratesQuery;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use color_eyre::Result;

/// Represents the parameters needed for fetching crates asynchronously.
pub struct SearchParameters {
    pub search: String,
    pub page: u64,
    pub page_size: u64,
    pub crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
    pub versions: Arc<Mutex<Vec<crates_io_api::Version>>>,
    pub loading_status: Arc<AtomicBool>,
    pub sort: crates_io_api::Sort,
    pub tx: UnboundedSender<Action>,
}

/// Performs the actual search, and sends the result back through the
/// sender.
pub async fn request_search_results(params: &SearchParameters) -> Result<(), String> {
    // Fetch crates using the created client with the error handling in one place.
    let client = create_client()?;
    let query = create_query(params);
    let (crates, versions, total) = fetch_crates_and_metadata(client, query).await?;
    update_state_with_fetched_crates(crates, versions, total, params);
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

async fn fetch_crates_and_metadata(
    client: crates_io_api::AsyncClient,
    query: crates_io_api::CratesQuery,
) -> Result<(Vec<crates_io_api::Crate>, Vec<crates_io_api::Version>, u64), String> {
    let page_result = client
        .crates(query)
        .await
        .map_err(|err| format!("API Client Error: {err:#?}"))?;
    let crates = page_result.crates;
    let total = page_result.meta.total;
    let versions = page_result.versions;

    Ok((crates, versions, total))
}

/// Handles the result after fetching crates and sending corresponding
/// actions.
fn update_state_with_fetched_crates(
    crates: Vec<crates_io_api::Crate>,
    versions: Vec<crates_io_api::Version>,
    total: u64,
    params: &SearchParameters,
) {
    // Lock and update the shared state container
    let mut app_crates = params.crates.lock().unwrap();
    app_crates.clear();
    app_crates.extend(crates);

    let mut app_versions = params.versions.lock().unwrap();
    app_versions.clear();
    app_versions.extend(versions);

    // After a successful fetch, send relevant actions based on the result
    if app_crates.is_empty() {
        let _ = params.tx.send(Action::ShowErrorPopup(format!(
            "Could not find any crates with query `{}`.",
            params.search
        )));
    } else {
        let _ = params.tx.send(Action::StoreTotalNumberOfCrates(total));
        let _ = params.tx.send(Action::Tick);
        let _ = params.tx.send(Action::ScrollDown);
    }
}

// Performs the async fetch of crate details.
pub async fn request_crate_details(
    crate_name: &str,
    crate_info: Arc<Mutex<Option<crates_io_api::CrateResponse>>>,
) -> Result<(), String> {
    let client = create_client()?;

    let crate_data = client
        .get_crate(crate_name)
        .await
        .map_err(|err| format!("Error fetching crate details: {err:#?}"))?;
    *crate_info.lock().unwrap() = Some(crate_data);
    Ok(())
}

// Performs the async fetch of crate details.
pub async fn request_full_crate_details(
    crate_name: &str,
    full_crate_info: Arc<Mutex<Option<crates_io_api::FullCrate>>>,
) -> Result<(), String> {
    let client = create_client()?;

    let full_crate_data = client
        .full_crate(crate_name, false)
        .await
        .map_err(|err| format!("Error fetching crate details: {err:#?}"))?;

    *full_crate_info.lock().unwrap() = Some(full_crate_data);
    Ok(())
}

pub async fn request_summary(
    summary: Arc<Mutex<Option<crates_io_api::Summary>>>,
) -> Result<(), String> {
    let client = create_client()?;

    let summary_data = client
        .summary()
        .await
        .map_err(|err| format!("Error fetching crate details: {err:#?}"))?;
    *summary.lock().unwrap() = Some(summary_data);
    Ok(())
}
