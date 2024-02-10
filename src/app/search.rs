#[derive(Debug)]
pub struct Search {
    /// A string for the current search input by the user, submitted to
    /// crates.io as a query
    pub search: String,
}

impl Search {
    pub fn new() -> Self {
        Self {
            search: String::new(),
        }
    }
}
