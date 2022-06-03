use std::string::String;
use std::{collections::HashMap, path::Path};

use crate::session::UserSession;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE},
    Response, StatusCode,
};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

static HOME_PAGE_URL: &str = "https://osu.ppy.sh/home";
static LOGIN_URL: &str = "https://osu.ppy.sh/session";

// This can be embed into normal function called
#[inline]
fn new_download_set_url(sid: u64) -> String {
    format!("https://osu.ppy.sh/beatmapsets/{sid}/download?noVideo=1")
}

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

/// 封装的get请求方法
async fn response_for_get(url: &str, headers: HeaderMap) -> Result<Response> {
    Ok(CLIENT.get(url).headers(headers).send().await?)
}

/// 封装的post请求
async fn response_for_post(
    url: &str,
    headers: HeaderMap,
    body: HashMap<String, &String>,
) -> Result<Response> {
    Ok(CLIENT.post(url).headers(headers).form(&body).send().await?)
}

/// 封装的下载请求
async fn response_for_download(url: &str, headers: HeaderMap) -> Result<Response> {
    let response = CLIENT.get(url).headers(headers).send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(OsuMapDownloadError::NotFoundMapError.into());
    }
    Ok(response)
}

/// 封装的请求头构造
fn get_download_header(id_str: &str, user: &mut UserSession) -> HeaderMap {
    let mut header = HeaderMap::new();
    header.insert(
        COOKIE,
        new_cookie(&user.token, &user.session)
            .parse()
            .unwrap(),
    );
    let back_url = format!("https://osu.ppy.sh/beatmapsets/{id_str}");
    header.insert("referer", back_url.parse().unwrap());
    header.insert(
        CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header
}

/// 生成请求用到的cookie字符串
#[inline]
fn new_cookie(xsrf: &str, cookie: &str) -> String {
    format!("XSRF-TOKEN={xsrf}; osu_session={cookie};")
}

/// 请求主页,用于得到及session记录
pub async fn visit_home(user: &mut UserSession) -> Result<()> {
    let mut header = HeaderMap::new();
    header.insert(
        "cookie",
        new_cookie(&user.token, &user.session)
            .parse()
            .unwrap_or_else(|_| HeaderValue::from(0)),
    );
    let response = response_for_get(HOME_PAGE_URL, header)
        .await
        .with_context(|| "请求主页失败")?;

    match response.status() {
        StatusCode::OK => {
            user.update(response.headers());
            Ok(())
        }
        StatusCode::BAD_REQUEST => Err(OsuMapDownloadError::IncorrectPasswordError.into()),
        _ => Err(OsuMapDownloadError::Unknown.into()),
    }
}

/// 更新登陆后的session
pub async fn login(user: &mut UserSession) -> Result<()> {
    let mut header = HeaderMap::new();
    header.insert("referer", HOME_PAGE_URL.parse()?);
    header.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header.insert(
        reqwest::header::COOKIE,
        new_cookie(&user.token, &user.session).parse()?,
    );

    let mut body = HashMap::new();
    body.insert("_token".to_string(), &user.token);
    body.insert("username".to_string(), &user.name);
    body.insert("password".to_string(), &user.password);

    let response = response_for_post(LOGIN_URL, header, body)
        .await
        .with_context(|| "登录请求无回复")?;

    match response.status() {
        StatusCode::OK => {
            user.update(response.headers());
            //pring_xxx(&f);
            Ok(())
        }
        StatusCode::FORBIDDEN => Err(OsuMapDownloadError::IncorrectPasswordError.into()),
        _ => Err(OsuMapDownloadError::Unknown.into()),
    }
}

/// 下载方法,使用 UserSession 信息下载
/// 如果短时间大量下载,尽可能使用不同的user下载
/// 使用Tokio以及reqwest依赖,确保版本匹配
pub async fn download(sid: u64, user: &mut UserSession, download_file_path: &Path) -> Result<()> {
    let url = new_download_set_url(sid);
    let sid = sid.to_string();
    let header = get_download_header(&sid, user);
    // 尝试使用已保存的session信息直接下载
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        return download_file(data, download_file_path, &sid).await;
    }
    // session 可能超时失效 ,进行刷新
    println!("刷新中");
    visit_home(user).await?;
    let header = get_download_header(&sid, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        return download_file(data, download_file_path, &sid).await;
    }
    // 重新登录
    println!("重新登录");
    login(user).await?;
    let header = get_download_header(&sid, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        return download_file(data, download_file_path, &sid).await;
    }
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
