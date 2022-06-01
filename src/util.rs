use bytes::Bytes;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE},
    Response, StatusCode,
};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::string::String;

static HOME_PAGE_URL: &str = "https://osu.ppy.sh/home";
static LOGIN_URL: &str = "https://osu.ppy.sh/session";
static DOWNLOAD_URL: &str = "https://osu.ppy.sh/beatmapsets/%s/download?noVideo=1";

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
    static ref REG_XSRF: Regex = Regex::new(r"XSRF-TOKEN=([\w\d]+);").unwrap();
    static ref REG_COOKIE: Regex = Regex::new(r"osu_session=([\w\d%]+);").unwrap();
}

#[derive(Debug, Clone)]
///自定义错误
pub struct OsuMapDownloadError {
    message: String,
}

impl Display for OsuMapDownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for OsuMapDownloadError {}

impl OsuMapDownloadError {
    pub fn new(msg: &str) -> Self {
        OsuMapDownloadError {
            message: String::from(msg),
        }
    }
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
}

/// 封装的get请求方法
async fn response_for_get(url: &str, headers: HeaderMap) -> Result<Response, Box<dyn Error>> {
    let p = CLIENT.get(url).headers(headers).send().await?;
    Ok(p)
}
/// 封装的post请求
async fn response_for_post(
    url: &str,
    headers: HeaderMap,
    body: HashMap<String, String>,
) -> Result<Response, Box<dyn Error>> {
    let p = CLIENT.post(url).headers(headers).form(&body).send().await?;
    Ok(p)
}
/// 封装的下载请求
async fn response_for_download(url: &str, headers: HeaderMap) -> Result<Response, Box<dyn Error>> {
    let p = CLIENT.get(url).headers(headers).send().await?;
    Ok(p)
}
/// 封装的请求头构造
fn get_download_header(id_str: &str, user: &mut UserSession) -> HeaderMap {
    let mut header = HeaderMap::new();
    let cookie_key = COOKIE;
    header.insert(
        cookie_key.clone(),
        format_cookie_str(&user.token, &user.session)
            .parse()
            .unwrap(),
    );
    let mut back_url = "https://osu.ppy.sh/beatmapsets/%s".to_string();
    back_url.push_str(id_str);
    header.insert("referer", back_url.parse().unwrap());
    header.insert(
        CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header
}
/// 封装的cookie解析
/// 自动更新session
fn get_new_cookie(header: &HeaderMap, user: &mut UserSession) -> (Option<String>, Option<String>) {
    let mut token: Option<String> = None;
    let mut cookie: Option<String> = None;
    let mut str;
    let all = header.get_all("set-cookie");
    for val in all {
        str = val.to_str().unwrap_or("");
        if let Some(xsrf) = REG_XSRF.captures(str) {
            if token.eq(&None) {
                token = Some(get_regex_one(xsrf));
                user.token = token.as_ref().unwrap().clone();
            }
        } else if let Some(cookie_match) = REG_COOKIE.captures(str) {
            if cookie.eq(&None) {
                cookie = Some(get_regex_one(cookie_match));
                user.session = cookie.as_ref().unwrap().clone();
            }
        }
        if !token.eq(&None) && !cookie.eq(&None) {
            return (token, cookie);
        }
    }

    (token, cookie)
}
/// 正则提取
fn get_regex_one(cap: Captures) -> String {
    cap.get(1).map_or("", |m| m.as_str()).to_string()
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
    format!("XSRF-TOKEN={}; osu_session={};", xsrf, cookie).to_string()
}
/// 请求主页,用于得到及session记录
async fn do_home(user: &mut UserSession) -> Result<(), OsuMapDownloadError> {
    let mut header = HeaderMap::new();
    header.insert(
        "cookie",
        format_cookie_str(&user.token, &user.session)
            .parse()
            .unwrap_or(HeaderValue::from(0)),
    );
    let response = response_for_get(HOME_PAGE_URL, header).await.unwrap();

    match response.status() {
        StatusCode::OK => {
            get_new_cookie(response.headers(), user);
            Ok(())
        }
        StatusCode::BAD_REQUEST => Err(OsuMapDownloadError::new("连接失败,检查网络")),
        _ => Err(OsuMapDownloadError::new("其他异常")),
    }
}
/// 更新登陆后的session
async fn do_login(user: &mut UserSession) -> Result<(), OsuMapDownloadError> {
    let mut header = HeaderMap::new();
    header.insert("referer", HOME_PAGE_URL.parse().unwrap());
    header.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    header.insert(
        reqwest::header::COOKIE,
        format_cookie_str(&user.token, &user.session)
            .parse()
            .unwrap(),
    );

    let mut body = HashMap::new();
    body.insert("_token".to_string(), user.token.clone());
    body.insert("username".to_string(), user.name.clone());
    body.insert("password".to_string(), user.password.to_string());

    let response = response_for_post(LOGIN_URL, header, body).await.unwrap();

    match response.status() {
        StatusCode::OK => {
            get_new_cookie(response.headers(), user);
            //pring_xxx(&f);
            Ok(())
        }
        StatusCode::FORBIDDEN => Err(OsuMapDownloadError::new("验证失败,检查是否密码错误")),
        _ => Err(OsuMapDownloadError::new("其他异常")),
    }
}
/// 下载方法,使用UserSession信息下载
/// 如果短时间大量下载,尽可能使用不同的user下载
/// 使用Tokio以及reqwest依赖,确保版本匹配
pub async fn do_download(sid: u64, user: &mut UserSession) -> Result<Bytes, Box<dyn Error>> {
    let id_str = sid.to_string();
    let url = DOWNLOAD_URL.to_string().replace("%s", &id_str);
    let header = get_download_header(&id_str, user);
    // 尝试使用已保存的session信息直接下载
    let data = response_for_download(&url, header).await?;
    if data.status().eq(&StatusCode::OK) {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // session 可能超时失效 ,进行刷新
    do_home(user).await?;
    let header = get_download_header(&id_str, user);
    let data = response_for_download(&url, header).await?;
    if data.status().eq(&StatusCode::OK) {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 重新登录
    do_login(user).await?;
    let header = get_download_header(&id_str, user);
    let data = response_for_download(&url, header).await?;
    if data.status().eq(&StatusCode::OK) {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 登录失败抛出错误
    Err(Box::new(OsuMapDownloadError::new("下载失败")))
}
