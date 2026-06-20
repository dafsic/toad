use std::convert::Infallible;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
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

    state.auth_store.write().await.insert(
        code.clone(),
        AuthSession {
            user_id: None,
            created_at: Instant::now(),
            tx: None, // filled later in wait_login
            token: None,
        },
    );

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

    // Background task: wait for bot verification or timeout.
    // On success we receive the token over the oneshot (internal), store it on the session,
    // and emit a non-secret "ready" signal. The actual JWT is delivered only via Set-Cookie
    // on a subsequent /complete call so it never becomes readable by page JavaScript.
    tokio::spawn(async move {
        tokio::select! {
            result = rx => {
                if let Ok(token) = result {
                    // Store token in session for the claim step and remove the oneshot sender
                    {
                        let mut store = state.auth_store.write().await;
                        if let Some(sess) = store.get_mut(&code) {
                            sess.token = Some(token);
                        }
                    }
                    let _ = stream_tx.send(Ok(Event::default().data(r#"{"ready":true}"#))).await;
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5 * 60)) => {
                tracing::debug!(code, "login wait timeout");
            }
        }
    });

    let stream = ReceiverStream::new(receiver);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// POST /api/auth/complete/:code — Claim the JWT and receive it via HttpOnly cookie.
///
/// Called by the frontend after receiving the "ready" signal from /wait.
/// The token is never exposed to JavaScript; it only travels in the Set-Cookie header.
/// The session is consumed on success.
pub async fn complete_login(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let token = {
        let mut store = state.auth_store.write().await;
        match store.remove(&code) {
            Some(session) if session.user_id.is_some() => session.token,
            _ => None,
        }
    };

    let token = token.ok_or((StatusCode::BAD_REQUEST, "Invalid or expired code".to_string()))?;

    let cookie = format!(
        "auth_token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800; Secure",
        token
    );

    let cookie_header: axum::http::HeaderValue = cookie.parse()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "bad cookie".to_string()))?;

    let resp = (StatusCode::NO_CONTENT, [(axum::http::header::SET_COOKIE, cookie_header)]).into_response();
    Ok(resp)
}
