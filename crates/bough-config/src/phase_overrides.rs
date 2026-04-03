use std::collections::HashMap;
use std::path::PathBuf;

use facet::Facet;

use crate::TimeoutConfig;

#[derive(Facet, Debug, Clone, Default)]
pub struct PhaseOverrides {
    #[facet(default)]
    pub pwd: Option<String>,

    #[facet(default)]
    pub env: Option<HashMap<String, String>>,

    #[facet(default)]
    pub timeout: Option<TimeoutConfig>,
}

impl PhaseOverrides {
    pub fn resolve_pwd(&self, global: &PhaseOverrides) -> PathBuf {
        self.pwd
            .as_deref()
            .or(global.pwd.as_deref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn resolve_env(&self, global: &PhaseOverrides) -> HashMap<String, String> {
        let mut result = global.env.clone().unwrap_or_default();
        if let Some(phase_env) = &self.env {
            for (k, v) in phase_env {
                if v.is_empty() {
                    result.remove(k);
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
        }
        result
    }

    pub fn resolve_timeout_absolute(&self, global: &PhaseOverrides) -> Option<chrono::Duration> {
        self.timeout
            .as_ref()
            .and_then(|t| t.absolute)
            .or_else(|| global.timeout.as_ref().and_then(|t| t.absolute))
            .map(|secs| chrono::Duration::seconds(secs as i64))
    }

    pub fn resolve_timeout_relative(&self, global: &PhaseOverrides) -> Option<f64> {
        self.timeout
            .as_ref()
            .and_then(|t| t.relative)
            .or_else(|| global.timeout.as_ref().and_then(|t| t.relative))
    }

    pub fn resolve_timeout(
        &self,
        global: &PhaseOverrides,
        reference: Option<chrono::Duration>,
    ) -> chrono::Duration {
        let absolute = self.resolve_timeout_absolute(global);
        let relative_multiplier = self.resolve_timeout_relative(global);
        let relative = match (relative_multiplier, reference) {
            (Some(multiplier), Some(ref_dur)) => Some(ref_dur * multiplier as i32),
            _ => None,
        };
        match (absolute, relative) {
            (Some(a), Some(r)) => std::cmp::min(a, r),
            (Some(a), None) => a,
            (None, Some(r)) => r,
            (None, None) => chrono::Duration::minutes(5),
        }
    }
}
