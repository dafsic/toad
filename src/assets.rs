use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// 将 `frontend/dist/` 目录内容在编译期嵌入二进制。
/// 构建前须先执行 `npm run build`（在 `frontend/` 目录下）。
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

/// Axum fallback handler：托管嵌入的前端静态资源。
///
/// - 精确路径匹配时直接返回对应文件（带正确 Content-Type）。
/// - 未找到时回退到 `index.html`，支持 React SPA 客户端路由。
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    serve_asset(path)
        .or_else(|| {
            // SPA 回退：非 API、非有扩展名的路径均走 index.html
            if !path.contains('.') {
                serve_asset("index.html")
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            (StatusCode::NOT_FOUND, "404 Not Found").into_response()
        })
}

fn serve_asset(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();
    let body = file.data.into_owned();
    Some(
        (
            [(header::CONTENT_TYPE, mime)],
            body,
        )
            .into_response(),
    )
}

