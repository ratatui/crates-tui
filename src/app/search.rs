#[derive(Debug)]
pub struct Search {
    /// A string for the current search input by the user, submitted to
    /// crates.io as a query
    pub search: String,

    /// A string for the current filter input by the user, used only locally
    /// for filtering for the list of crates in the current view.
    pub filter: String,
}

impl Search {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            filter: String::new(),
        }
    }
}
