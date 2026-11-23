//! Why a queued write is not being delivered. Writes in one credential scope are sent in order,
//! so a single request the server rejects for good keeps every later one waiting.
use super::{Scope, retry, state};
use crate::api::Api;
use crate::request_outbox::{self, PendingRequest};
use serde_json::{Value, json};

pub struct Blockage {
    /// The oldest write ahead of the one just queued.
    pub request: PendingRequest,
    pub ahead: usize,
    /// The server's answer when it rejected `request` for good.
    pub rejection: Option<String>,
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

/// What to do about a write the server will not accept as it is.
pub fn repair(request: &PendingRequest) -> String {
    let id = short(&request.id);
    let rebase = if request_outbox::rebase::base_field(request).is_some() {
        format!(
            "If its base revision is outdated, read the current record and run cj outbox rebase {id} --base VERSION. "
        )
    } else {
        String::new()
    };
    format!(
        "Inspect it with cj outbox list --body. {rebase}To discard only that write, run cj outbox drop {id}. Then run cj sync."
    )
}

/// The write the automatic worker gave up on, while it is still the oldest one in its scope.
pub fn rejected(scope: &Scope) -> Option<(PendingRequest, String)> {
    let saved = state::load(scope).ok()?;
    let request = scope.writes().ok()?.into_iter().next()?;
    (saved.blocked_request.as_deref() == Some(request.id.as_str()))
        .then(|| (request, saved.last_error.unwrap_or_default()))
}

/// The writes ahead of the one just queued, when there are any.
pub fn ahead_of(api: &Api, tenant: &str, queued_id: &str) -> Option<Blockage> {
    let scope = Scope::new(api, tenant);
    let writes = scope.writes().ok()?;
    let ahead = writes.iter().position(|request| request.id == queued_id)?;
    let request = writes.into_iter().next().filter(|_| ahead > 0)?;
    let rejection = rejected(&scope)
        .filter(|(rejected, _)| rejected.id == request.id)
        .map(|(_, error)| error.chars().take(300).collect());
    Some(Blockage {
        request,
        ahead,
        rejection,
    })
}

impl Blockage {
    pub fn json(&self) -> Value {
        json!({"request_id": self.request.id, "method": self.request.method,
            "path": self.request.path, "ahead": self.ahead, "rejected": self.rejection.is_some(),
            "error": self.rejection, "repair": self.rejection.as_ref().map(|_| repair(&self.request))})
    }

    /// Replaces the usual "retries when online" advice when delivery is stuck.
    pub fn text(&self) -> Option<String> {
        let error = self.rejection.as_ref()?;
        Some(format!(
            "It is not being delivered: the server rejected an earlier queued write, {} ({} {}), and {} write(s) are ahead of this one. Server answer: {error}. {}",
            short(&self.request.id),
            self.request.method,
            self.request.path,
            self.ahead,
            repair(&self.request)
        ))
    }
}

/// Adds the repair steps to an explicit sync that stopped on a write the server will not accept.
pub fn explain(api: &Api, error: anyhow::Error) -> anyhow::Error {
    if retry::retry_delay(&error, 0, "").is_some() {
        return error;
    }
    let blocking = request_outbox::entries().ok().and_then(|entries| {
        entries
            .into_iter()
            .map(|(_, request)| request)
            .find(|request| request.server == api.server())
    });
    match blocking {
        Some(request) => error.context(format!(
            "the outbox is blocked by write {}. {}",
            short(&request.id),
            repair(&request)
        )),
        None => error,
    }
}
