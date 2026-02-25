use super::contracts::{AnalyticsError, ConnectorConfigAttestationV1};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

const SIGNATURE_PREFIX: &str = "ed25519:";
const PAYLOAD_SCHEMA_V1: &str = "attestation-v1";
const ENV_PRIVATE_KEY_B64: &str = "ATTESTATION_ED25519_PRIVATE_KEY";
const ENV_KEY_ID: &str = "ATTESTATION_ED25519_KEY_ID";

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
