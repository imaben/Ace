use ace_core::{AceError, AceResult, AwsAuthOptions};
use chrono::Utc;
use sha2::{Digest, Sha256};

/// Generate an RDS IAM authentication token.
///
/// This implements the AWS SigV4 query-string signing for
/// `rds-db:connect` without pulling the full AWS SDK (keeps compile light).
/// For production SSO / MFA flows, swap in `aws-config` + `aws-sigv4`.
pub async fn generate_rds_iam_token(
    host: &str,
    port: u16,
    user: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    session_token: Option<&str>,
) -> AceResult<String> {
    let date = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = &date[..8];
    let credential_scope = format!("{date_stamp}/{region}/rds-db/aws4_request");

    let mut query = format!(
        "Action=connect&DBUser={}&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential={}%2F{}&X-Amz-Date={}&X-Amz-Expires=900&X-Amz-SignedHeaders=host",
        urlencoding_encode(user),
        urlencoding_encode(access_key),
        urlencoding_encode(&credential_scope),
        date
    );
    if let Some(token) = session_token {
        query.push_str("&X-Amz-Security-Token=");
        query.push_str(&urlencoding_encode(token));
    }

    let canonical_request = format!(
        "GET\n/\n{query}\nhost:{host}:{port}\n\nhost\n{}",
        sha256_hex("")
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{date}\n{credential_scope}\n{}",
        sha256_hex(&canonical_request)
    );

    let signing_key = derive_signing_key(secret_key, date_stamp, region, "rds-db");
    let signature = hmac_sha256_hex(&signing_key, string_to_sign.as_bytes());
    query.push_str("&X-Amz-Signature=");
    query.push_str(&signature);

    Ok(format!("{host}:{port}/?{query}"))
}

pub async fn resolve_aws_token(
    opts: &AwsAuthOptions,
    host: &str,
    port: u16,
    user: &str,
) -> AceResult<String> {
    if !opts.enabled {
        return Err(AceError::Auth("AWS auth not enabled".into()));
    }
    let region = opts
        .region
        .clone()
        .or_else(|| std::env::var("AWS_REGION").ok())
        .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
        .unwrap_or_else(|| "us-east-1".into());

    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| AceError::Auth("AWS_ACCESS_KEY_ID not set".into()))?;
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| AceError::Auth("AWS_SECRET_ACCESS_KEY not set".into()))?;
    let session_token = std::env::var("AWS_SESSION_TOKEN").ok();

    generate_rds_iam_token(
        host,
        port,
        user,
        &region,
        &access_key,
        &secret_key,
        session_token.as_deref(),
    )
    .await
}

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use sha2::Sha256;
    // Manual HMAC to avoid extra hmac crate
    let mut key = key.to_vec();
    if key.len() > 64 {
        let mut hasher = Sha256::new();
        hasher.update(&key);
        key = hasher.finalize().to_vec();
    }
    if key.len() < 64 {
        key.resize(64, 0);
    }
    let mut ipad = vec![0x36u8; 64];
    let mut opad = vec![0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut hasher = Sha256::new();
    hasher.update(&ipad);
    hasher.update(data);
    let inner = hasher.finalize();
    let mut hasher = Sha256::new();
    hasher.update(&opad);
    hasher.update(inner);
    hasher.finalize().to_vec()
}

fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    hex::encode(hmac_sha256(key, data))
}

fn derive_signing_key(secret: &str, datestamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), datestamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}