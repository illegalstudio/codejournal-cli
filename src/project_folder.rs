use crate::{api::Api, attribution, checkout_identity};
use std::path::PathBuf;

/// How the current directory maps to a Code Journal project.
pub enum Scope {
    /// A named project: `CJ_PROJECT` or a registered folder containing this directory.
    Named(String),
    /// A Git checkout, resolved through its remote or registered path.
    Checkout,
    /// Neither a Git checkout nor a registered folder.
    Outside,
    /// The server could not be asked.
    Unknown,
}

pub fn scope(api: &Api, tenant: &str) -> Scope {
    if let Ok(slug) = std::env::var("CJ_PROJECT") {
        return Scope::Named(slug);
    }
    if checkout_identity::current().is_some() {
        return Scope::Checkout;
    }
    registered(api, tenant)
}

/// Finds the registered project folder that contains the current directory.
pub fn registered(api: &Api, tenant: &str) -> Scope {
    let Some(cwd) = current_dir() else {
        return Scope::Outside;
    };
    let path = cwd.to_string_lossy();
    let Ok(query) = reqwest::Url::parse_with_params(
        "http://local/",
        [
            ("host", attribution::host().as_str()),
            ("path", &path),
            ("within", "1"),
        ],
    ) else {
        return Scope::Unknown;
    };
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/resolve?{}",
        query.query().unwrap_or("")
    );
    match api.get(&endpoint) {
        Ok(found) => found["project"]["slug"]
            .as_str()
            .map_or(Scope::Outside, |slug| Scope::Named(slug.to_owned())),
        Err(error) if error.to_string().contains("404") => Scope::Outside,
        Err(_) => Scope::Unknown,
    }
}

pub fn current_dir() -> Option<PathBuf> {
    std::env::current_dir().ok()?.canonicalize().ok()
}

/// Session context for a directory without a project, replacing the brief.
pub fn notice() -> String {
    let cwd = current_dir().unwrap_or_default();
    format!(
        "Code Journal: no project here. {} is not inside a Git repository or a folder registered \
         with `cj project init`, so this session has no journal. Do not run cj or write journal \
         records in this session. Run `cj project init` only when the user asks to register this \
         folder, and never for temporary or scratch directories.",
        cwd.display()
    )
}
