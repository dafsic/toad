use rust_embed::RustEmbed;
use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};

/// 将 frontend/dist/ 目录内容在编译期嵌入二进制。
/// 构建前须先执行 `npm run build`（在 frontend/ 目录下）。
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

/// Axum fallback handler，托管前端静态资源。
/// 所有未匹配 /api/* 的请求均由此处理，支持 SPA 路由（回退至 index.html）。
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    // TODO:
    // 1. 从 uri.path() 提取文件路径（去掉前导 /）
    // 2. Assets::get(path) 查找嵌入文件
    // 3. 若未找到，回退至 Assets::get("index.html")（SPA 路由支持）
    // 4. 根据 mime_guess 设置 Content-Type header
    // 5. 返回文件内容
    todo!()
}
