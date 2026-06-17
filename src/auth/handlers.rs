use std::convert::Infallible;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Json,
};
use futures::Stream;
use serde::Serialize;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;

use crate::api::AppState;
use crate::auth::{generate_code, AuthSession};

/// POST /api/auth/request 响应
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    /// 6位验证码，用户需发送给 Telegram Bot
    pub code: String,
}

/// SSE /api/auth/wait/{code} 成功响应
#[derive(Debug, Serialize)]
struct LoginSuccess {
    /// JWT token
    token: String,
}

/// POST /api/auth/request — 生成验证码
///
/// 前端调用此接口获取 6 位验证码，并展示给用户。
/// 返回的验证码有效期 5 分钟。
pub async fn request_login(State(state): State<AppState>) -> Json<LoginRequest> {
    let code = generate_code();

    tracing::info!(code, "generated login code");

    state
        .auth_store
        .write()
        .await
        .insert(code.clone(), AuthSession {
            code: code.clone(),
            user_id: None,
            created_at: Instant::now(),
            tx: None, // 稍后在 wait_login 中填充
        });

    Json(LoginRequest { code })
}

/// GET /api/auth/wait/{code} — SSE 等待验证
///
/// 前端建立 SSE 连接，等待 Telegram Bot 验证成功。
/// Bot 验证后会通过 oneshot channel 发送 token。
pub async fn wait_login(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // 创建 oneshot channel
    let (tx, rx) = oneshot::channel();

    // 检查验证码是否存在，并设置 tx
    {
        let mut store = state.auth_store.write().await;
        let session = store.get_mut(&code).ok_or(StatusCode::NOT_FOUND)?;

        // 如果已有 tx，说明已有其他 SSE 连接在等待（拒绝重复连接）
        if session.tx.is_some() {
            return Err(StatusCode::CONFLICT);
        }

        session.tx = Some(tx);
    } // 释放锁

    // 创建 channel 用于 stream
    let (stream_tx, receiver) = tokio::sync::mpsc::channel(1);

    // 后台任务：等待验证或超时
    tokio::spawn(async move {
        tokio::select! {
            // 等待 Bot 验证成功
            result = rx => {
                if let Ok(token) = result {
                    let success = LoginSuccess { token };
                    let data = serde_json::to_string(&success).unwrap_or_default();
                    let _ = stream_tx.send(Ok(Event::default().data(data))).await;
                }
            }
            // 5 分钟超时
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5 * 60)) => {
                tracing::debug!(code, "login wait timeout");
            }
        }
    });

    let stream = ReceiverStream::new(receiver);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// 从 Cookie 中提取并验证 token
pub async fn verify_auth(state: &AppState, cookie_header: Option<&str>) -> Result<u64, StatusCode> {
    let cookie_str = cookie_header.ok_or(StatusCode::UNAUTHORIZED)?;
    let token = crate::auth::extract_token_from_cookie(cookie_str)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = crate::auth::verify_token(&token, &state.config.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 验证 user_id 是否匹配配置
    if claims.sub != state.config.allowed_telegram_user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(claims.sub)
}
