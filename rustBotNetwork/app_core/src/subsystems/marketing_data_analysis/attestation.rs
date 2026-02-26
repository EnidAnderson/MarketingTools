use super::contracts::{AnalyticsError, ConnectorConfigAttestationV1};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SIGNATURE_PREFIX: &str = "ed25519:";
const PAYLOAD_SCHEMA_V1: &str = "attestation-v1";
const ENV_PRIVATE_KEY_B64: &str = "ATTESTATION_ED25519_PRIVATE_KEY";
const ENV_KEY_ID: &str = "ATTESTATION_ED25519_KEY_ID";
const ENV_KEY_REGISTRY_JSON: &str = "ATTESTATION_KEY_REGISTRY_JSON";
const ENV_KEY_REGISTRY_PATH: &str = "ATTESTATION_KEY_REGISTRY_PATH";

#[derive(Debug, Clone, Default)]
pub struct AttestationKeyRegistryV1 {
    by_key_id: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationRegistryDiagnosticsV1 {
    pub signature_present: bool,
    pub key_id_present: bool,
    pub key_id: Option<String>,
    pub registry_configured: bool,
    pub registry_key_count: usize,
    pub key_id_found_in_registry: Option<bool>,
}

impl AttestationKeyRegistryV1 {
    pub fn from_json_str(raw: &str) -> Result<Self, AnalyticsError> {
        let parsed: HashMap<String, String> = serde_json::from_str(raw).map_err(|_| {
            AnalyticsError::validation(
                "attestation_registry_invalid",
                "ATTESTATION_KEY_REGISTRY_JSON must be a JSON object of {key_id: base64_pubkey}",
                "connector_attestation.fingerprint_key_id",
            )
        })?;
        if parsed.is_empty() {
            return Err(AnalyticsError::validation(
                "attestation_registry_empty",
                "attestation key registry cannot be empty",
                "connector_attestation.fingerprint_key_id",
            ));
        }
        Ok(Self { by_key_id: parsed })
    }

    pub fn from_file(path: &Path) -> Result<Self, AnalyticsError> {
        let raw = fs::read_to_string(path).map_err(|err| {
            AnalyticsError::internal(
                "attestation_registry_read_failed",
                format!("failed to read attestation key registry file: {err}"),
            )
        })?;
        Self::from_json_str(raw.trim())
    }

    pub fn lookup_public_key_b64(&self, key_id: &str) -> Option<&str> {
        self.by_key_id.get(key_id).map(String::as_str)
    }

    pub fn key_count(&self) -> usize {
        self.by_key_id.len()
    }

    pub fn has_key_id(&self, key_id: &str) -> bool {
        self.by_key_id.contains_key(key_id)
    }
}

pub fn load_attestation_key_registry_from_env_or_file(
) -> Result<Option<AttestationKeyRegistryV1>, AnalyticsError> {
    if let Some(raw_json) = std::env::var(ENV_KEY_REGISTRY_JSON)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return AttestationKeyRegistryV1::from_json_str(&raw_json).map(Some);
    }

    if let Some(path_str) = std::env::var(ENV_KEY_REGISTRY_PATH)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return AttestationKeyRegistryV1::from_file(Path::new(&path_str)).map(Some);
    }

    Ok(None)
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::attestation`
/// purpose: Build canonical payload bytes for signing and verification.
/// invariants:
///   - Field order and separators are stable.
///   - Payload schema marker is pinned to `attestation-v1`.
pub fn canonical_attestation_payload_v1(
    run_id: &str,
    artifact_id: &str,
    attestation: &ConnectorConfigAttestationV1,
) -> Result<Vec<u8>, AnalyticsError> {
    let created_at = attestation
        .fingerprint_created_at
        .as_deref()
        .ok_or_else(|| {
            AnalyticsError::validation(
                "attestation_created_at_missing",
                "fingerprint_created_at is required to sign attestation",
                "connector_attestation.fingerprint_created_at",
            )
        })?
        .trim();
    if created_at.is_empty() {
        return Err(AnalyticsError::validation(
            "attestation_created_at_missing",
            "fingerprint_created_at is required to sign attestation",
            "connector_attestation.fingerprint_created_at",
        ));
    }

    Ok(format!(
        "{PAYLOAD_SCHEMA_V1}\nrun_id={}\nartifact_id={}\ncreated_at={}\nmode={}\nfingerprint_alg={}\nfingerprint_schema={}\nfingerprint={}\nruntime_build={}\n",
        run_id.trim(),
        artifact_id.trim(),
        created_at,
        attestation.connector_mode_effective.trim(),
        attestation.fingerprint_alg.trim(),
        attestation.fingerprint_input_schema.trim(),
        attestation.connector_config_fingerprint.trim(),
        attestation.runtime_build.as_deref().unwrap_or("").trim(),
    )
    .into_bytes())
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::attestation`
/// purpose: Optionally sign connector attestation with ed25519 if signing env vars are configured.
/// invariants:
///   - If private key env var is absent, signing is skipped.
///   - If private key env var is present, key id must be present and signature must be written.
pub fn maybe_sign_connector_attestation_v1(
    run_id: &str,
    artifact_id: &str,
    attestation: &mut ConnectorConfigAttestationV1,
) -> Result<(), AnalyticsError> {
    let Some((signing_key, key_id)) = maybe_load_signing_key_from_env()? else {
        return Ok(());
    };
    if attestation_is_empty(attestation) {
        return Ok(());
    }
    if attestation.connector_mode_effective.trim().is_empty()
        || attestation.connector_config_fingerprint.trim().is_empty()
        || attestation.fingerprint_alg.trim().is_empty()
        || attestation.fingerprint_input_schema.trim().is_empty()
    {
        return Err(AnalyticsError::validation(
            "attestation_fields_incomplete",
            "connector attestation fields must be populated before signing",
            "connector_attestation",
        ));
    }

    let payload = canonical_attestation_payload_v1(run_id, artifact_id, attestation)?;
    let signature = signing_key.sign(&payload);
    attestation.fingerprint_signature = Some(format!(
        "{SIGNATURE_PREFIX}{}",
        STANDARD_NO_PAD.encode(signature.to_bytes())
    ));
    attestation.fingerprint_key_id = Some(key_id);
    Ok(())
}

fn attestation_is_empty(attestation: &ConnectorConfigAttestationV1) -> bool {
    attestation.connector_mode_effective.trim().is_empty()
        && attestation.connector_config_fingerprint.trim().is_empty()
        && attestation.fingerprint_alg.trim().is_empty()
        && attestation.fingerprint_input_schema.trim().is_empty()
        && attestation.fingerprint_salt_id.is_none()
        && attestation.fingerprint_signature.is_none()
        && attestation.fingerprint_key_id.is_none()
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::attestation`
/// purpose: Verify connector attestation signature for audit and ingestion checks.
pub fn verify_connector_attestation_signature_v1(
    run_id: &str,
    artifact_id: &str,
    attestation: &ConnectorConfigAttestationV1,
    public_key_b64: &str,
) -> Result<(), AnalyticsError> {
    let signature_str = attestation
        .fingerprint_signature
        .as_deref()
        .ok_or_else(|| {
            AnalyticsError::validation(
                "attestation_signature_missing",
                "fingerprint_signature is required for verification",
                "connector_attestation.fingerprint_signature",
            )
        })?
        .trim();
    let signature_b64 = signature_str
        .strip_prefix(SIGNATURE_PREFIX)
        .ok_or_else(|| {
            AnalyticsError::validation(
                "attestation_signature_format_invalid",
                "fingerprint_signature must start with 'ed25519:'",
                "connector_attestation.fingerprint_signature",
            )
        })?;
    let signature_bytes = STANDARD_NO_PAD.decode(signature_b64).map_err(|_| {
        AnalyticsError::validation(
            "attestation_signature_format_invalid",
            "fingerprint_signature is not valid base64",
            "connector_attestation.fingerprint_signature",
        )
    })?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| {
        AnalyticsError::validation(
            "attestation_signature_format_invalid",
            "fingerprint_signature byte length is invalid",
            "connector_attestation.fingerprint_signature",
        )
    })?;

    let public_key_bytes = STANDARD.decode(public_key_b64.trim()).map_err(|_| {
        AnalyticsError::validation(
            "attestation_public_key_invalid",
            "public key must be base64-encoded ed25519 verifying key",
            "connector_attestation.fingerprint_key_id",
        )
    })?;
    let public_key_array: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
        AnalyticsError::validation(
            "attestation_public_key_invalid",
            "public key must decode to 32 bytes",
            "connector_attestation.fingerprint_key_id",
        )
    })?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_array).map_err(|_| {
        AnalyticsError::validation(
            "attestation_public_key_invalid",
            "public key bytes are not a valid ed25519 key",
            "connector_attestation.fingerprint_key_id",
        )
    })?;
    let payload = canonical_attestation_payload_v1(run_id, artifact_id, attestation)?;
    verifying_key.verify(&payload, &signature).map_err(|_| {
        AnalyticsError::validation(
            "attestation_signature_invalid",
            "connector attestation signature verification failed",
            "connector_attestation.fingerprint_signature",
        )
    })
}

pub fn verify_connector_attestation_with_registry_v1(
    run_id: &str,
    artifact_id: &str,
    attestation: &ConnectorConfigAttestationV1,
    registry: &AttestationKeyRegistryV1,
) -> Result<(), AnalyticsError> {
    let key_id = attestation
        .fingerprint_key_id
        .as_deref()
        .ok_or_else(|| {
            AnalyticsError::validation(
                "attestation_signature_key_id_mismatch",
                "fingerprint_key_id is required when fingerprint_signature is present",
                "connector_attestation.fingerprint_key_id",
            )
        })?
        .trim();
    if key_id.is_empty() {
        return Err(AnalyticsError::validation(
            "attestation_signature_key_id_mismatch",
            "fingerprint_key_id is required when fingerprint_signature is present",
            "connector_attestation.fingerprint_key_id",
        ));
    }
    let public_key_b64 = registry.lookup_public_key_b64(key_id).ok_or_else(|| {
        AnalyticsError::validation(
            "attestation_unknown_key_id",
            "fingerprint_key_id is not present in attestation registry",
            "connector_attestation.fingerprint_key_id",
        )
    })?;
    verify_connector_attestation_signature_v1(run_id, artifact_id, attestation, public_key_b64)
}

pub fn attestation_registry_diagnostics_v1(
    attestation: &ConnectorConfigAttestationV1,
    registry: Option<&AttestationKeyRegistryV1>,
) -> AttestationRegistryDiagnosticsV1 {
    let signature_present = attestation
        .fingerprint_signature
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let key_id = attestation
        .fingerprint_key_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let key_id_present = key_id.is_some();

    let (registry_configured, registry_key_count, key_id_found_in_registry) = match registry {
        Some(registry) => {
            let found = key_id.as_deref().map(|value| registry.has_key_id(value));
            (true, registry.key_count(), found)
        }
        None => (false, 0, None),
    };

    AttestationRegistryDiagnosticsV1 {
        signature_present,
        key_id_present,
        key_id,
        registry_configured,
        registry_key_count,
        key_id_found_in_registry,
    }
}

pub fn attestation_registry_validation_message_v1(
    diagnostics: &AttestationRegistryDiagnosticsV1,
) -> String {
    format!(
        "signed attestations must resolve key_id in registry and verify signature (signature_present={}, key_id_present={}, key_id={}, registry_configured={}, registry_key_count={}, key_id_found={})",
        diagnostics.signature_present,
        diagnostics.key_id_present,
        diagnostics.key_id.as_deref().unwrap_or("-"),
        diagnostics.registry_configured,
        diagnostics.registry_key_count,
        diagnostics
            .key_id_found_in_registry
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    )
}

fn maybe_load_signing_key_from_env() -> Result<Option<(SigningKey, String)>, AnalyticsError> {
    let Some(private_key_b64) = std::env::var(ENV_PRIVATE_KEY_B64)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let key_id = std::env::var(ENV_KEY_ID)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AnalyticsError::validation(
                "attestation_key_id_missing",
                "ATTESTATION_ED25519_KEY_ID is required when signing key is configured",
                "connector_attestation.fingerprint_key_id",
            )
        })?;

    let key_bytes = STANDARD.decode(private_key_b64).map_err(|_| {
        AnalyticsError::validation(
            "attestation_private_key_invalid",
            "ATTESTATION_ED25519_PRIVATE_KEY must be base64 bytes",
            "connector_attestation.fingerprint_signature",
        )
    })?;

    let secret_key = match key_bytes.len() {
        32 => {
            let seed: [u8; 32] = key_bytes.try_into().map_err(|_| {
                AnalyticsError::validation(
                    "attestation_private_key_invalid",
                    "ATTESTATION_ED25519_PRIVATE_KEY must decode to 32-byte seed or 64-byte keypair",
                    "connector_attestation.fingerprint_signature",
                )
            })?;
            SigningKey::from_bytes(&seed)
        }
        64 => {
            let mut seed = [0_u8; 32];
            seed.copy_from_slice(&key_bytes[..32]);
            SigningKey::from_bytes(&seed)
        }
        _ => {
            return Err(AnalyticsError::validation(
                "attestation_private_key_invalid",
                "ATTESTATION_ED25519_PRIVATE_KEY must decode to 32-byte seed or 64-byte keypair",
                "connector_attestation.fingerprint_signature",
            ));
        }
    };
    Ok(Some((secret_key, key_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: once_cell::sync::Lazy<Mutex<()>> =
        once_cell::sync::Lazy::new(|| Mutex::new(()));

    fn sample_attestation() -> ConnectorConfigAttestationV1 {
        ConnectorConfigAttestationV1 {
            connector_mode_effective: "observed_read_only".to_string(),
            connector_config_fingerprint: "sha256:abc123".to_string(),
            fingerprint_alg: "sha256".to_string(),
            fingerprint_input_schema: "connector-config-v1".to_string(),
            fingerprint_created_at: Some("2026-02-25T20:00:00Z".to_string()),
            runtime_build: Some("1229112".to_string()),
            fingerprint_salt_id: None,
            fingerprint_signature: None,
            fingerprint_key_id: None,
        }
    }

    #[test]
    fn canonical_payload_is_stable() {
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &sample_attestation())
                .expect("payload");
        let text = String::from_utf8(payload).expect("utf8");
        assert!(text.starts_with("attestation-v1\nrun_id=run-1\n"));
        assert!(text.contains("\nmode=observed_read_only\n"));
        assert!(text.ends_with("\nruntime_build=1229112\n"));
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let mut att = sample_attestation();
        let seed = [7_u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let pubkey_b64 = STANDARD.encode(signing_key.verifying_key().as_bytes());
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("test-key".to_string());

        let verify =
            verify_connector_attestation_signature_v1("run-1", "artifact-1", &att, &pubkey_b64);
        assert!(verify.is_ok());
    }

    #[test]
    fn verify_fails_on_tampered_mode() {
        let mut att = sample_attestation();
        let seed = [3_u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let pubkey_b64 = STANDARD.encode(signing_key.verifying_key().as_bytes());
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("test-key".to_string());
        att.connector_mode_effective = "simulated".to_string();

        let verify =
            verify_connector_attestation_signature_v1("run-1", "artifact-1", &att, &pubkey_b64);
        assert!(verify.is_err());
    }

    #[test]
    fn verify_with_registry_passes_for_known_key_id() {
        let mut att = sample_attestation();
        let seed = [11_u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("k1".to_string());
        let registry = AttestationKeyRegistryV1::from_json_str(&format!(
            r#"{{"k1":"{}"}}"#,
            STANDARD.encode(signing_key.verifying_key().as_bytes())
        ))
        .expect("registry");

        let verify =
            verify_connector_attestation_with_registry_v1("run-1", "artifact-1", &att, &registry);
        assert!(verify.is_ok());
    }

    #[test]
    fn verify_with_registry_fails_for_unknown_key_id() {
        let mut att = sample_attestation();
        let seed = [12_u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("missing".to_string());
        let registry = AttestationKeyRegistryV1::from_json_str(&format!(
            r#"{{"k1":"{}"}}"#,
            STANDARD.encode(signing_key.verifying_key().as_bytes())
        ))
        .expect("registry");

        let verify =
            verify_connector_attestation_with_registry_v1("run-1", "artifact-1", &att, &registry);
        assert_eq!(
            verify.expect_err("must fail").code,
            "attestation_unknown_key_id"
        );
    }

    #[test]
    fn verify_with_registry_fails_when_key_material_rotates_without_key_id_change() {
        let mut att = sample_attestation();
        let old_signing_key = SigningKey::from_bytes(&[21_u8; 32]);
        let new_signing_key = SigningKey::from_bytes(&[22_u8; 32]);
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = old_signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("prod-key".to_string());

        let rotated_registry = AttestationKeyRegistryV1::from_json_str(&format!(
            r#"{{"prod-key":"{}"}}"#,
            STANDARD.encode(new_signing_key.verifying_key().as_bytes())
        ))
        .expect("registry");

        let verify = verify_connector_attestation_with_registry_v1(
            "run-1",
            "artifact-1",
            &att,
            &rotated_registry,
        );
        assert_eq!(
            verify.expect_err("must fail").code,
            "attestation_signature_invalid"
        );
    }

    #[test]
    fn verify_with_registry_passes_during_rotation_window_when_old_key_still_registered() {
        let mut att = sample_attestation();
        let old_signing_key = SigningKey::from_bytes(&[31_u8; 32]);
        let new_signing_key = SigningKey::from_bytes(&[32_u8; 32]);
        let payload =
            canonical_attestation_payload_v1("run-1", "artifact-1", &att).expect("payload");
        let sig = old_signing_key.sign(&payload);
        att.fingerprint_signature = Some(format!(
            "{}{}",
            SIGNATURE_PREFIX,
            STANDARD_NO_PAD.encode(sig.to_bytes())
        ));
        att.fingerprint_key_id = Some("key-2026-02".to_string());

        let dual_registry = AttestationKeyRegistryV1::from_json_str(&format!(
            r#"{{"key-2026-02":"{}","key-2026-03":"{}"}}"#,
            STANDARD.encode(old_signing_key.verifying_key().as_bytes()),
            STANDARD.encode(new_signing_key.verifying_key().as_bytes())
        ))
        .expect("registry");

        let verify = verify_connector_attestation_with_registry_v1(
            "run-1",
            "artifact-1",
            &att,
            &dual_registry,
        );
        assert!(verify.is_ok());
    }

    #[test]
    fn registry_diagnostics_report_configuration_and_key_resolution() {
        let mut att = sample_attestation();
        att.fingerprint_signature = Some("ed25519:abc".to_string());
        att.fingerprint_key_id = Some("missing".to_string());

        let no_registry = attestation_registry_diagnostics_v1(&att, None);
        assert!(no_registry.signature_present);
        assert!(no_registry.key_id_present);
        assert!(!no_registry.registry_configured);
        assert_eq!(no_registry.registry_key_count, 0);
        assert!(no_registry.key_id_found_in_registry.is_none());

        let registry = AttestationKeyRegistryV1::from_json_str(
            r#"{"k1":"AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="}"#,
        )
        .expect("registry");
        let with_registry = attestation_registry_diagnostics_v1(&att, Some(&registry));
        assert!(with_registry.registry_configured);
        assert_eq!(with_registry.registry_key_count, 1);
        assert_eq!(with_registry.key_id_found_in_registry, Some(false));
    }

    #[test]
    fn maybe_sign_skips_when_key_absent() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(&[(ENV_PRIVATE_KEY_B64, None), (ENV_KEY_ID, None)], || {
            let mut att = sample_attestation();
            let result = maybe_sign_connector_attestation_v1("run-1", "artifact-1", &mut att);
            assert!(result.is_ok());
            assert!(att.fingerprint_signature.is_none());
            assert!(att.fingerprint_key_id.is_none());
        });
    }

    #[test]
    fn maybe_sign_requires_key_id_when_private_key_present() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(
            &[
                (
                    ENV_PRIVATE_KEY_B64,
                    Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="),
                ),
                (ENV_KEY_ID, None),
            ],
            || {
                let mut att = sample_attestation();
                let result = maybe_sign_connector_attestation_v1("run-1", "artifact-1", &mut att);
                assert!(result.is_err());
                assert_eq!(
                    result.expect_err("must fail").code,
                    "attestation_key_id_missing"
                );
            },
        );
    }

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
}
