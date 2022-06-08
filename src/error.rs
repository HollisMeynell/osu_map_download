use thiserror::Error;

/// 不同类型的错误，在网络请求失败时使用
#[derive(Debug, Clone, Error, PartialEq)]
pub enum OsuMapDownloadError {
    #[error("验证失败,检查是否密码错误")]
    IncorrectPasswordError,
    #[error("没有找到该谱面,或者已经下架或被删除,无法下载")]
    NotFoundMapError,
    #[error("登录失败")]
    LoginFailError,
    #[error("请求下载失败，可能是 Cookie 过期")]
    DownloadRequestError,
    #[error("文件大小未知，可能出现网络问题")]
    UnknownSizeError,
    #[error("无法创建下载文件路径：{path:?}，错误：{error:?}")]
    TargetFileCreationError {
        path: String,
        error: String,
    },
    #[error("无法写入下载文件：{path:?}，错误：{error:?}")]
    TargetFileWriteError {
        path: String,
        error: String,
    },
    #[error("网络出错，文件下载中断")]
    DownloadPartError,
    #[error("其他异常")]
    Unknown,
}
