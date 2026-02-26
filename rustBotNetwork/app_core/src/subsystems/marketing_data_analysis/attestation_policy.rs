use super::contracts::AnalyticsError;

const ENV_REQUIRE_SIGNED_ATTESTATIONS: &str = "REQUIRE_SIGNED_ATTESTATIONS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationPolicySourceV1 {
    ProfileDefault,
    EnvOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttestationPolicyV1 {
    pub require_signed_attestations: bool,
    pub source: AttestationPolicySourceV1,
}

impl AttestationPolicyV1 {
    pub fn explain(&self) -> &'static str {
        match self.source {
            AttestationPolicySourceV1::ProfileDefault => "default_production_profile",
            AttestationPolicySourceV1::EnvOverride => "env_override",
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::attestation_policy`
/// purpose: Resolve signed-attestation enforcement policy from profile + optional env override.
/// invariants:
///   - Invalid env values fail closed with validation error.
///   - If env is unset, production-like profiles require signed attestations.
pub fn resolve_attestation_policy_v1(
    profile_id: &str,
) -> Result<AttestationPolicyV1, AnalyticsError> {
    match env_bool(ENV_REQUIRE_SIGNED_ATTESTATIONS)? {
        Some(value) => Ok(AttestationPolicyV1 {
            require_signed_attestations: value,
            source: AttestationPolicySourceV1::EnvOverride,
        }),
        None => Ok(AttestationPolicyV1 {
            require_signed_attestations: is_production_profile_like(profile_id),
            source: AttestationPolicySourceV1::ProfileDefault,
        }),
    }
}

pub fn is_production_profile_like(profile_id: &str) -> bool {
    let normalized = profile_id.trim().to_ascii_lowercase();
    normalized == "production"
        || normalized == "prod"
        || normalized.starts_with("production-")
        || normalized.starts_with("production_")
        || normalized.starts_with("prod-")
        || normalized.starts_with("prod_")
}

fn env_bool(key: &str) -> Result<Option<bool>, AnalyticsError> {
    let Some(raw) = std::env::var(key).ok() else {
        return Ok(None);
    };
    let normalized = raw.trim().to_ascii_lowercase();
    let value = match normalized.as_str() {
        "1" | "true" | "yes" => true,
        "0" | "false" | "no" => false,
        _ => {
            return Err(AnalyticsError::validation(
                "attestation_policy_invalid_env",
                "REQUIRE_SIGNED_ATTESTATIONS must be one of true/false/1/0/yes/no",
                "REQUIRE_SIGNED_ATTESTATIONS",
            ))
        }
    };
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn with_temp_env<F>(pairs: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        let previous = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        f();
        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }

    #[test]
    fn resolver_matrix_unset_prod_requires() {
        let _guard = ENV_MUTEX.lock().expect("env lock");
        with_temp_env(&[(ENV_REQUIRE_SIGNED_ATTESTATIONS, None)], || {
            let p = resolve_attestation_policy_v1("production-us").expect("policy");
            assert!(p.require_signed_attestations);
            assert_eq!(p.source, AttestationPolicySourceV1::ProfileDefault);
        });
    }

    #[test]
    fn resolver_matrix_unset_dev_not_required() {
        let _guard = ENV_MUTEX.lock().expect("env lock");
        with_temp_env(&[(ENV_REQUIRE_SIGNED_ATTESTATIONS, None)], || {
            let p = resolve_attestation_policy_v1("dev").expect("policy");
            assert!(!p.require_signed_attestations);
            assert_eq!(p.source, AttestationPolicySourceV1::ProfileDefault);
        });
    }

    #[test]
    fn resolver_matrix_true_any_profile_requires() {
        let _guard = ENV_MUTEX.lock().expect("env lock");
        with_temp_env(&[(ENV_REQUIRE_SIGNED_ATTESTATIONS, Some("true"))], || {
            let p = resolve_attestation_policy_v1("dev").expect("policy");
            assert!(p.require_signed_attestations);
            assert_eq!(p.source, AttestationPolicySourceV1::EnvOverride);
        });
    }

    #[test]
    fn resolver_matrix_false_any_profile_not_required() {
        let _guard = ENV_MUTEX.lock().expect("env lock");
        with_temp_env(&[(ENV_REQUIRE_SIGNED_ATTESTATIONS, Some("false"))], || {
            let p = resolve_attestation_policy_v1("production").expect("policy");
            assert!(!p.require_signed_attestations);
            assert_eq!(p.source, AttestationPolicySourceV1::EnvOverride);
        });
    }

    #[test]
    fn resolver_matrix_invalid_env_errors() {
        let _guard = ENV_MUTEX.lock().expect("env lock");
        with_temp_env(&[(ENV_REQUIRE_SIGNED_ATTESTATIONS, Some("maybe"))], || {
            let err = resolve_attestation_policy_v1("dev").expect_err("must fail");
            assert_eq!(err.code, "attestation_policy_invalid_env");
        });
    }
}
