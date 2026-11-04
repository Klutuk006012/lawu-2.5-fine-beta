// ============================================================
// auth_middleware.rs — Shared auth module (Lawu-2.5-Fine v3.6.0)
// ------------------------------------------------------------
// Included via #[path = "../auth_middleware.rs"] in each binary
// (auth_api.rs / chat_api.rs / monitor_api.rs).
//
// Fungsi:
//   - verify_token(headers) -> Result<AuthUser, (StatusCode, Json<Value>)>
//     Mendeteksi: Authorization: Bearer lawu_<api_key>  (api key)
//                 Authorization: Bearer <jwt>            (JWT HS256)
//   - require_admin(user) -> Result<(), (StatusCode, Json<Value>)>
//   - Shared SQLite connection via Arc<Mutex<Connection>>
//
// Catatan:
//   - JWT secret: env LAWU_JWT_SECRET atau file /var/lib/lawu/jwt_secret
//     (auto-generate 32 byte sekali saat first run).
//   - TTL 24 jam, claim: { sub: user_id, email, role, exp }.
//   - rusqlite sync -> Axum async, jadi SEMUA query harus dibungkus
//     tokio::task::spawn_blocking oleh caller.
// ============================================================

use axum::{
    http::{HeaderMap, StatusCode},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;

// ------------------------------ Konstanta ------------------------------
pub const AUTH_DB_PATH_DEFAULT: &str = "/var/lib/lawu/auth.db";
pub const JWT_SECRET_PATH_DEFAULT: &str = "/var/lib/lawu/jwt_secret";
pub const JWT_TTL_SECONDS: i64 = 24 * 60 * 60; // 24 jam
pub const API_KEY_PREFIX: &str = "lawu_";

/// Resolve auth DB path (env override untuk sandbox test).
pub fn auth_db_path() -> String {
    std::env::var("LAWU_AUTH_DB_PATH").unwrap_or_else(|_| AUTH_DB_PATH_DEFAULT.to_string())
}

/// Resolve jwt secret file path (env override untuk sandbox test).
pub fn jwt_secret_path() -> String {
    std::env::var("LAWU_JWT_SECRET_PATH").unwrap_or_else(|_| JWT_SECRET_PATH_DEFAULT.to_string())
}

// ------------------------------ Tipe ------------------------------
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtClaims {
    pub sub: i64, // user_id
    pub email: String,
    pub role: String,
    pub exp: usize, // unix seconds
}

pub type DbPool = Arc<Mutex<Connection>>;

// ------------------------------ JWT secret ------------------------------
pub fn get_jwt_secret() -> Vec<u8> {
    // 1. env override
    if let Ok(s) = std::env::var("LAWU_JWT_SECRET") {
        if !s.is_empty() {
            return s.into_bytes();
        }
    }
    // 2. persisted file
    let secret_path = jwt_secret_path();
    if let Ok(bytes) = std::fs::read(&secret_path) {
        if bytes.len() >= 32 {
            return bytes;
        }
    }
    // 3. generate new (first run)
    let _ = std::fs::create_dir_all(std::path::Path::new(&secret_path).parent().unwrap_or(std::path::Path::new("/var/lib/lawu")));
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut buf = [0u8; 32];
    rng.fill_bytes(&mut buf);
    let _ = std::fs::write(&secret_path, &buf);
    // allow world read untuk dev env (lawu user shares)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&secret_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o640);
            let _ = std::fs::set_permissions(&secret_path, perms);
        }
    }
    buf.to_vec()
}

// ------------------------------ Encode / decode JWT ------------------------------
pub fn issue_jwt(user: &AuthUser) -> Result<String, String> {
    let secret = get_jwt_secret();
    let exp = (chrono::Utc::now().timestamp() + JWT_TTL_SECONDS) as usize;
    let claims = JwtClaims {
        sub: user.id,
        email: user.email.clone(),
        role: user.role.clone(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .map_err(|e| format!("gagal issue jwt: {e}"))
}

pub fn verify_jwt(token: &str) -> Result<JwtClaims, String> {
    let secret = get_jwt_secret();
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(&secret),
        &validation,
    )
    .map_err(|e| format!("jwt invalid: {e}"))?;
    Ok(data.claims)
}

// ------------------------------ API key helper ------------------------------
pub fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn is_api_key(token: &str) -> bool {
    token.starts_with(API_KEY_PREFIX)
}

// ------------------------------ verify_token (HTTP header) ------------------------------
// Auto-detect: "Bearer lawu_<key>" (api key) vs "Bearer <jwt>"
pub async fn verify_token(headers: &HeaderMap) -> Result<AuthUser, (StatusCode, Json<Value>)> {
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Authorization header hilang" })),
            )
        })?;
    let token = if let Some(rest) = bearer.strip_prefix("Bearer ") {
        rest.trim()
    } else if let Some(rest) = bearer.strip_prefix("bearer ") {
        rest.trim()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Authorization harus 'Bearer <token>'" })),
        ));
    };
    if token.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "token kosong" })),
        ));
    }

    if is_api_key(token) {
        // API key path — look up by hash, but caller must supply DB pool.
        // We cannot access DB without pool here, so we return Err with hint
        // to call verify_token_with_db instead.
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "api key diberikan tapi db pool tidak tersedia — panggil verify_token_with_db" })),
        ));
    }

    // JWT path
    match verify_jwt(token) {
        Ok(c) => Ok(AuthUser {
            id: c.sub,
            email: c.email,
            role: c.role,
        }),
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": format!("token tidak valid: {e}") })),
        )),
    }
}

// verify_token WITH DB pool (auto-detect JWT vs API key)
pub async fn verify_token_with_db(
    headers: &HeaderMap,
    pool: DbPool,
) -> Result<AuthUser, (StatusCode, Json<Value>)> {
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Authorization header hilang" })),
            )
        })?;
    let token = if let Some(rest) = bearer.strip_prefix("Bearer ") {
        rest.trim()
    } else if let Some(rest) = bearer.strip_prefix("bearer ") {
        rest.trim()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Authorization harus 'Bearer <token>'" })),
        ));
    };
    if token.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "token kosong" })),
        ));
    }

    if is_api_key(token) {
        // API key path: hash + lookup
        let key_hash = sha256_hex(token);
        let pool_clone = pool.clone();
        let res = tokio::task::spawn_blocking(move || -> Result<Option<AuthUser>, String> {
            let conn = pool_clone.blocking_lock();
            let mut stmt = conn
                .prepare(
                    "SELECT u.id, u.email, u.role FROM api_keys ak \
                     JOIN users u ON u.id = ak.user_id \
                     WHERE ak.key_hash = ?1 AND ak.revoked_at IS NULL",
                )
                .map_err(|e| format!("db prepare: {e}"))?;
            let row = stmt
                .query_row([&key_hash], |r| {
                    Ok(AuthUser {
                        id: r.get(0)?,
                        email: r.get(1)?,
                        role: r.get(2)?,
                    })
                })
                .ok();
            // bump last_used_at
            let _ = conn.execute(
                "UPDATE api_keys SET last_used_at = ?1 WHERE key_hash = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), &key_hash],
            );
            Ok(row)
        })
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn_blocking: {e}") })),
            )
        })?;
        match res {
            Ok(Some(u)) => Ok(u),
            Ok(None) => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "api key tidak valid / sudah direvoke" })),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("db: {e}") })),
            )),
        }
    } else {
        // JWT path
        match verify_jwt(token) {
            Ok(c) => Ok(AuthUser {
                id: c.sub,
                email: c.email,
                role: c.role,
            }),
            Err(e) => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": format!("token tidak valid: {e}") })),
            )),
        }
    }
}

// ------------------------------ require_admin ------------------------------
pub fn require_admin(user: &AuthUser) -> Result<(), (StatusCode, Json<Value>)> {
    if user.role == "admin" {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "akses khusus admin" })),
        ))
    }
}

// ------------------------------ Validation helpers ------------------------------
pub fn is_valid_email(s: &str) -> bool {
    let at = match s.find('@') {
        Some(i) => i,
        None => return false,
    };
    let after_at = &s[at + 1..];
    after_at.contains('.') && !after_at.starts_with('.') && at > 0
}

pub fn is_strong_password(pw: &str) -> bool {
    pw.chars().count() >= 8
}
