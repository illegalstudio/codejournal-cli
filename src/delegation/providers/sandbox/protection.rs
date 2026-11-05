use serde_json::{Value, json};
use std::{path::Path, process::Command};

pub struct Protection {
    pub report: Value,
    pub scratch: Option<tempfile::TempDir>,
    pub marker: Option<tempfile::NamedTempFile>,
    pub(super) wrapper: Option<Command>,
}

impl Protection {
    pub fn disabled(reason: &str) -> Self {
        let reason = crate::secret_redaction::text(reason).0;
        let reason = reason.chars().take(2048).collect::<String>();
        Self {
            report: json!({"status":"disabled","enforced":false,"backend":null,"reason":reason}),
            scratch: None,
            marker: None,
            wrapper: None,
        }
    }

    pub fn redact(&mut self, session: &Value) {
        for field in ["reason", "previous_failure"] {
            if let Some(value) = self.report[field].as_str() {
                let mut clean = crate::secret_redaction::text(value).0;
                for secret in [
                    super::super::super::text(session, "attempt_token").to_owned(),
                    std::env::var("CJ_DELEGATION_TOKEN").unwrap_or_default(),
                    std::env::var("CJ_TOKEN").unwrap_or_default(),
                ] {
                    if secret.len() >= 8 {
                        clean = clean.replace(&secret, "[redacted credential]");
                    }
                }
                self.report[field] = json!(clean.chars().take(2048).collect::<String>());
            }
        }
    }

    pub fn apply(&mut self, command: &mut Command, client: &Path) {
        if let Some(root) = &self.scratch {
            command
                .env("TMPDIR", root.path().join("tmp"))
                .env("XDG_CACHE_HOME", root.path().join("cache"))
                .env("XDG_RUNTIME_DIR", root.path().join("runtime"));
        }
        command.env(
            "CJ_DELEGATION_SANDBOX",
            if self.wrapper.is_some() { "1" } else { "0" },
        );
        let Some(mut wrapper) = self.wrapper.take() else {
            return;
        };
        if let Some(marker) = &self.marker {
            wrapper
                .arg(client)
                .arg("__cj_sandbox_exec")
                .arg(marker.path());
        }
        wrapper.arg(command.get_program()).args(command.get_args());
        for (key, value) in command.get_envs() {
            match value {
                Some(value) => {
                    wrapper.env(key, value);
                }
                None => {
                    wrapper.env_remove(key);
                }
            }
        }
        if let Some(cwd) = command.get_current_dir() {
            wrapper.current_dir(cwd);
        }
        for name in [
            "DBUS_SESSION_BUS_ADDRESS",
            "SSH_AUTH_SOCK",
            "DISPLAY",
            "WAYLAND_DISPLAY",
        ] {
            wrapper.env_remove(name);
        }
        if let Some(root) = &self.scratch {
            wrapper
                .env("TMPDIR", root.path().join("tmp"))
                .env("XDG_CACHE_HOME", root.path().join("cache"))
                .env("XDG_RUNTIME_DIR", root.path().join("runtime"));
            crate::config::credential_scope::forward(&mut wrapper);
        }
        wrapper.env("PYTHONDONTWRITEBYTECODE", "1");
        *command = wrapper;
    }
}
