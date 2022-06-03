use std::path::Path;

use crate::user::UserSession;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use reqwest::{Response, StatusCode};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::error::OsuMapDownloadError;

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

/// 封装的下载请求
async fn try_download(sid: &str, user: &UserSession, path: &Path) -> Result<()> {
    let url = format!("https://osu.ppy.sh/beatmapsets/{sid}/download?noVideo=1");
    let headers = user.new_header(&sid);

    let response = CLIENT.get(url).headers(headers).send().await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(OsuMapDownloadError::NotFoundMapError.into());
    }

    if response.status() == StatusCode::OK {
        return download_file(response, path, &sid).await;
    }

    Ok(())
}

/// 下载方法,使用 UserSession 信息下载
/// 如果短时间大量下载,尽可能使用不同的user下载
/// 使用Tokio以及reqwest依赖,确保版本匹配
pub async fn download(sid: u64, user: &mut UserSession, download_file_path: &Path) -> Result<()> {
    let sid = sid.to_string();

    let res = try_download(&sid, user, download_file_path).await;

    // FIXME: We should distinguish error before retry. There are some error indicate that there is
    // no need to retry.
    if res.is_ok() {
        return Ok(())
    }

    // session 可能超时失效 ,进行刷新
    println!("Fail to download, try refreshing..");
    user.refresh().await?;

    try_download(&sid, user, download_file_path).await?;

    // 登录失败抛出错误
    Err(OsuMapDownloadError::LoginFailError.into())
}

async fn download_file(resp: Response, write_to: &Path, sid: &str) -> Result<()> {
    let total_size = resp
        .content_length()
        .ok_or_else(|| anyhow::anyhow!("无法获取文件大小"))?;

    let filename = write_to.file_name().unwrap().to_str().unwrap();
    let bar = ProgressBar::new(total_size);
    bar.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    bar.set_message(format!("正在下载谱面 {sid}"));
    let mut file = File::create(write_to).await?;
    let mut downloaded = 0;
    let mut resp_stream = resp.bytes_stream();

    while let Some(chunk) = resp_stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)
            .await
            .with_context(|| "下载文件时出现错误")?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        bar.set_position(new);
    }

    bar.finish_with_message(format!("谱面下载完成，保存到: {filename}"));
    Ok(())
}
