use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub mod handlers;
pub mod middleware;

/// JWT Claims 结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Telegram user_id
    pub sub: u64,
    /// 过期时间（UNIX 时间戳）
    pub exp: i64,
    /// 签发时间
    pub iat: i64,
}

/// 登录会话（用于验证码等待期间）
pub struct AuthSession {
    /// 验证成功后的 Telegram user_id
    pub user_id: Option<u64>,
    /// 创建时间（5分钟后自动过期）
    pub created_at: Instant,
    /// 通知前端的 channel（SSE 使用）
    pub tx: Option<tokio::sync::oneshot::Sender<String>>,
}

/// 全局会话存储（验证码 → 会话）
pub type AuthStore = Arc<RwLock<HashMap<String, AuthSession>>>;

/// 创建空的认证存储
pub fn create_store() -> AuthStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// 生成 6 位随机验证码
pub fn generate_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

/// 生成 JWT token（8小时有效期）
pub fn generate_token(user_id: u64, secret: &str) -> anyhow::Result<String> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        exp: now + 8 * 3600, // 8小时
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("generate JWT token")
}

/// 验证并解析 JWT token
pub fn verify_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .context("decode JWT token")?;

    Ok(token_data.claims)
}

/// 从 Cookie header 中提取 token
pub fn extract_token_from_cookie(cookie_header: &str) -> Option<String> {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix("auth_token=") {
            return Some(value.to_string());
        }
    }
    None
}

/// 定期清理过期的验证码会话（5分钟）
pub async fn cleanup_expired_sessions(store: AuthStore) {
    const SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        let mut sessions = store.write().await;
        let now = Instant::now();

        sessions.retain(|code, session| {
            let expired = now.duration_since(session.created_at) > SESSION_TIMEOUT;
            if expired {
                tracing::debug!(code, "cleaning up expired auth session");
            }
            !expired
        });
    }
}
