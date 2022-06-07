use std::path::{Path, PathBuf};

use crate::user::UserSession;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::HeaderMap;
use reqwest::{Response, StatusCode};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::client;
use crate::error::OsuMapDownloadError;

/// 封装的下载请求
async fn try_download(
    sid: &[String],
    user: &UserSession,
    path: &Path,
    no_video: bool,
) -> Result<(), OsuMapDownloadError> {
    // build a pair of the url and related headers
    let requisite: Vec<(String, String, HeaderMap)> = sid
        .iter()
        .map(|s| {
            (
                s.clone(),
                format!(
                    "https://osu.ppy.sh/beatmapsets/{s}/download?noVideo={}",
                    if no_video { "" } else { "1" }
                ),
                user.new_header(&s),
            )
        })
        .collect();

    // send the download request concurrently
    let mut tasks = Vec::with_capacity(requisite.len());
    for (sid, url, headers) in requisite {
        let t = tokio::spawn(async move {
            (
                sid.clone(),
                client::get(&url, headers.clone())
                    .await
                    .map_err(|_| OsuMapDownloadError::DownloadRequestError),
            )
        });
        tasks.push(t);
    }

    // write the response to disk concurrently
    let mut write_task = Vec::with_capacity(tasks.capacity());
    for handle in tasks {
        let path = path.to_owned();
        let (sid, response) = handle.await.expect("执行下载任务时发生了意料之外的错误");
        match response {
            Ok(resp) => {
                if resp.status() == StatusCode::NOT_FOUND {
                    eprintln!("{}", OsuMapDownloadError::NotFoundMapError);
                    continue;
                }

                if resp.status() == StatusCode::OK {
                    write_task.push(tokio::spawn(async move {
                        write_file(resp, path.to_path_buf(), sid)
                    }));
                }
            }
            Err(e) => {
                eprintln!("{e}");
            }
        }
    }

    Ok(())
}

/// 下载方法,使用 UserSession 信息下载
/// 如果短时间大量下载,尽可能使用不同的user下载
/// 使用Tokio以及reqwest依赖,确保版本匹配
pub async fn download(
    sid: &[String],
    user: &mut UserSession,
    download_file_path: &Path,
    no_video: bool,
) -> Result<()> {
    let res = try_download(sid, user, download_file_path, no_video).await;

    // match response. If return is Ok, we return ok.
    // If return is download request error, we refresh cookie and retry download
    // If return is other error, return error String.
    match res {
        Ok(_) => return Ok(()),
        Err(e) if e == OsuMapDownloadError::DownloadRequestError => (),
        Err(e) => anyhow::bail!("{}", e),
    }

    // session 可能超时失效 ,进行刷新
    println!("Fail to download, try refreshing..");
    user.refresh().await?;

    try_download(sid, user, download_file_path, no_video).await?;

    // 登录失败抛出错误
    Err(OsuMapDownloadError::LoginFailError.into())
}

/// Write the response to file with stream. Require reqwest::Response, path to write file, and the
/// unique set id. The file will be write to: {write_to}/sid.zip.
async fn write_file(
    resp: Response,
    mut prefix: PathBuf,
    sid: String,
) -> Result<(), OsuMapDownloadError> {
    let total_size = resp
        .content_length()
        .ok_or_else(|| OsuMapDownloadError::UnknownSizeError)?;

    prefix.push(format!("{sid}.osz"));
    let path = prefix.to_str().expect("非法路径名").to_string();
    let bar = ProgressBar::new(total_size);
    bar.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    bar.set_message(format!("正在下载谱面 {sid}"));
    let mut file =
        File::create(prefix)
            .await
            .map_err(|e| OsuMapDownloadError::TargetFileCreationError {
                path: path.clone(),
                error: e.to_string(),
            })?;
    let mut downloaded = 0;
    let mut resp_stream = resp.bytes_stream();

    while let Some(chunk) = resp_stream.next().await {
        let chunk = chunk.map_err(|_| OsuMapDownloadError::DownloadPartError)?;
        file.write_all(&chunk)
            .await
            .with_context(|| "下载文件时出现错误")
            .map_err(|e| OsuMapDownloadError::TargetFileWriteError {
                path: path.clone(),
                error: e.to_string(),
            })?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        bar.set_position(new);
    }

    bar.finish_with_message(format!("谱面下载完成，保存到: {path}"));
    Ok(())
}
