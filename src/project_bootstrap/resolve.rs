use crate::{api::Api, api::response_error::ResponseError, request_outbox};
use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde_json::Value;
use std::{thread, time::Duration};

pub fn checkout(api: &Api, endpoint: &str, tenant: &str, slug: &str) -> Result<Value> {
    let mut result = api.get(endpoint);
    for delay in [150, 350] {
        if !unregistered(&result) {
            return result;
        }
        thread::sleep(Duration::from_millis(delay));
        result = api.get(endpoint);
    }
    if !unregistered(&result) {
        return result;
    }
    let projects = format!("/api/v1/tenants/{tenant}/projects");
    if request_outbox::has_project_init(api.server(), &projects, slug)? {
        result.context("Checkout registration is queued. Synchronize it with cj sync, then retry")
    } else {
        result.context("This Git checkout has not resolved to a Code Journal project. If registration just started, retry shortly. Otherwise run cj project init here or select an existing project with --project SLUG")
    }
}

fn unregistered(result: &Result<Value>) -> bool {
    result
        .as_ref()
        .err()
        .and_then(|error| error.downcast_ref::<ResponseError>())
        .is_some_and(|error| error.status == StatusCode::NOT_FOUND)
}
