use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::api::AppState;

/// 认证中间件：验证 JWT token
///
/// 从 Cookie 中提取 `auth_token`，验证其有效性。
/// 验证失败返回 401 Unauthorized。
pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从 Cookie header 读取 token
    let cookie_header = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok());

    let token = cookie_header
        .and_then(crate::auth::extract_token_from_cookie)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 token
    let claims = crate::auth::verify_token(&token, &state.config.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 验证 user_id 是否匹配配置
    if claims.sub != state.config.allowed_telegram_user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // 通过验证，继续处理请求
    Ok(next.run(req).await)
}
