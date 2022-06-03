use thiserror::Error;

/// 不同类型的错误，在网络请求失败时使用
#[derive(Debug, Clone, Error)]
pub enum OsuMapDownloadError {
    #[error("验证失败,检查是否密码错误")]
    IncorrectPasswordError,
    #[error("没有找到该谱面,或者已经下架或被删除,无法下载")]
    NotFoundMapError,
    #[error("登录失败")]
    LoginFailError,
    #[error("其他异常")]
    Unknown,
}
