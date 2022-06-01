use anyhow::{Context, Result};
use bytes::Bytes;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE},
    Response, StatusCode,
};
use std::collections::HashMap;
use std::string::String;
use thiserror::Error;

static HOME_PAGE_URL: &str = "https://osu.ppy.sh/home";
static LOGIN_URL: &str = "https://osu.ppy.sh/session";

// This can be embed into normal function called
#[inline]
fn new_download_set_url(sid: u64) -> String {
    format!("https://osu.ppy.sh/beatmapsets/{sid}/download?noVideo=1")
}

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
    static ref REG_XSRF: Regex = Regex::new(r"XSRF-TOKEN=([\w\d]+);").unwrap();
    static ref REG_COOKIE: Regex = Regex::new(r"osu_session=([\w\d%]+);").unwrap();
}

/// 不同类型的错误，在网络请求失败时使用
#[derive(Debug, Clone, Error)]
pub enum OsuMapDownloadError {
    #[error("验证失败,检查是否密码错误")]
    IncorrectPassword,
    #[error("登录失败")]
    LoginFail,
    #[error("连接失败，检查网络")]
    NetworkError,
    #[error("其他异常")]
    Unknown,
}

/// 用户信息记录,包含密码,登录后的session
/// 包含的session信息可重用,请重用此结构
/// 可以将session保存出来
pub struct UserSession {
    name: String,
    password: String,
    token: String,
    session: String,
}

impl UserSession {
    /// 通过账号密码生产记录
    pub fn new(username: &str, password: &str) -> Self {
        UserSession {
            name: username.to_string(),
            password: password.to_string(),
            token: String::new(),
            session: String::new(),
        }
    }

    /// 保存当前session
    pub fn save_session(&mut self) -> String {
        format!("{}&{}", self.token, self.session)
    }
    /// 通过保存的session数据恢复
    pub fn read_session(&mut self, data: &str) {
        let reg = Regex::new(r"([\w\d]+)&([\w\d%]+)").unwrap();

        if let Some(s) = reg.captures(data) {
            self.token = s.get(1).unwrap().as_str().to_string();
            self.session = s.get(2).unwrap().as_str().to_string();
        }
    }

    // 更新 token 和 session。如果传入的 HeaderMap 没有满足更新的值，旧的值会保留
    pub fn update(&mut self, header_map: &HeaderMap) {
        let all_headers = header_map.get_all("set-cookie");
        let (mut xsrf_change, mut cookie_change) = (false, false);

        for header in all_headers {
            let str = header.to_str();
            if str.is_err() {
                // empty header, just enter into next loop
                continue;
            }
            let str = str.unwrap();

            // FIXME: This condition can be rewrite into if let chain after Rust 1.63 released
            if !xsrf_change {
                if let Some(xsrf) = REG_XSRF.captures(str) {
                    let old_token = &self.token;
                    self.token = xsrf
                        .get(1)
                        .map_or_else(|| old_token.clone(), |v| v.as_str().to_string());
                    xsrf_change = true;

                    // if header is parsed by REG_XSRF, we don't need to pass it into REG_COOKIE
                    // So here we just continue to another loop
                    continue;
                }
            } else if !cookie_change {
                if let Some(cookie_match) = REG_COOKIE.captures(str) {
                    let old_session = &self.session;
                    self.session = cookie_match
                        .get(1)
                        .map_or_else(|| old_session.clone(), |v| v.as_str().to_string());
                    cookie_change = true;
                }
            }

            if xsrf_change && cookie_change {
                break;
            }
        }
    }
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
    Ok(CLIENT.get(url).headers(headers).send().await?)
}

/// 封装的请求头构造
fn get_download_header(id_str: &str, user: &mut UserSession) -> HeaderMap {
    let mut header = HeaderMap::new();
    let cookie_key = COOKIE;
    header.insert(
        cookie_key,
        format_cookie_str(&user.token, &user.session)
            .parse()
            .unwrap(),
    );
    let mut back_url = "https://osu.ppy.sh/beatmapsets/".to_string();
    back_url.push_str(id_str);
    header.insert("referer", back_url.parse().unwrap());
    header.insert(
        CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header
}

#[test]
fn test_print_xxx() {
    /// debug方法,暂留
    fn print_xxx(f: &(Option<String>, Option<String>)) {
        if let Some(s) = &f.0 {
            println!("1:'{}'", s);
        } else {
            println!("1 is none!")
        }
        if let Some(s) = &f.1 {
            println!("2:'{}'", s);
        } else {
            println!("2 is none!")
        }
    }

    print_xxx(&(None, None))
}

/// 生成请求用到的cookie字符串
fn format_cookie_str(xsrf: &str, cookie: &str) -> String {
    format!("XSRF-TOKEN={xsrf}; osu_session={cookie};")
}

/// 请求主页,用于得到及session记录
async fn do_home(user: &mut UserSession) -> Result<()> {
    let mut header = HeaderMap::new();
    header.insert(
        "cookie",
        format_cookie_str(&user.token, &user.session)
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
        StatusCode::BAD_REQUEST => Err(OsuMapDownloadError::IncorrectPassword.into()),
        _ => Err(OsuMapDownloadError::Unknown.into()),
    }
}
/// 更新登陆后的session
async fn do_login(user: &mut UserSession) -> Result<()> {
    let mut header = HeaderMap::new();
    header.insert("referer", HOME_PAGE_URL.parse()?);
    header.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header.insert(
        reqwest::header::COOKIE,
        format_cookie_str(&user.token, &user.session).parse()?,
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
        StatusCode::FORBIDDEN => Err(OsuMapDownloadError::IncorrectPassword.into()),
        _ => Err(OsuMapDownloadError::Unknown.into()),
    }
}
/// 下载方法,使用UserSession信息下载
/// 如果短时间大量下载,尽可能使用不同的user下载
/// 使用Tokio以及reqwest依赖,确保版本匹配
pub async fn do_download(sid: u64, user: &mut UserSession) -> Result<Bytes> {
    let url = new_download_set_url(sid);
    let sid = sid.to_string();
    let header = get_download_header(&sid, user);
    // 尝试使用已保存的session信息直接下载
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // session 可能超时失效 ,进行刷新
    do_home(user).await?;
    let header = get_download_header(&sid, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 重新登录
    do_login(user).await?;
    let header = get_download_header(&sid, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 登录失败抛出错误
    Err(OsuMapDownloadError::LoginFail.into())
}
