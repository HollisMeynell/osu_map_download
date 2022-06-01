use bytes::Bytes;
use lazy_static::lazy_static;
use regex::Regex;
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

    // 更新 token 和 session。如果传入的 HeaderMap 没有满足更新的值，旧的值会保留
    pub fn update(&mut self, header_map: &HeaderMap) {
        let all_headers = header_map.get_all("set-cookie");
        for header in all_headers {
            let str = header.to_str();
            // early return to save regexp match time
            if str.is_err() {
                continue;
            }
            // it is safe to unwrap now
            let str = str.unwrap();
            if let Some(xsrf) = REG_XSRF.captures(str) {
                // 如果正则解析出了新的值，则更新值，否则把原来的值放进去。
                // 因为字符串拷贝是个开销很大的操作，所以这里先拿了一个原值的引用
                // 然后用 map_or_else 来懒惰执行。用 closure 之后只有在遇到 None 的时候，
                // old_token.clone() 才会被执行，于是我们当遇到 Some 的时候我们可以减少
                // 一次字符串拷贝的开销。
                let old_token = &self.token;
                self.token = xsrf
                    .get(1)
                    .map_or_else(|| old_token.clone(), |v| v.as_str().to_string());
            } else if let Some(cookie_match) = REG_COOKIE.captures(str) {
                // same as above
                let old_session = &self.session;
                self.session = cookie_match
                    .get(1)
                    .map_or_else(|| old_session.clone(), |v| v.as_str().to_string())
            }
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
    body: HashMap<String, &String>,
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
        cookie_key,
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
async fn do_home(user: &mut UserSession) -> Result<(), OsuMapDownloadError> {
    let mut header = HeaderMap::new();
    header.insert(
        "cookie",
        format_cookie_str(&user.token, &user.session)
            .parse()
            .unwrap_or_else(|_| HeaderValue::from(0)),
    );
    let response = response_for_get(HOME_PAGE_URL, header).await.unwrap();

    match response.status() {
        StatusCode::OK => {
            user.update(response.headers());
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
    body.insert("_token".to_string(), &user.token);
    body.insert("username".to_string(), &user.name);
    body.insert("password".to_string(), &user.password);

    let response = response_for_post(LOGIN_URL, header, body).await.unwrap();

    match response.status() {
        StatusCode::OK => {
            user.update(response.headers());
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
    let url = new_download_set_url(sid);
    let header = get_download_header(&id_str, user);
    // 尝试使用已保存的session信息直接下载
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // session 可能超时失效 ,进行刷新
    do_home(user).await?;
    let header = get_download_header(&id_str, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 重新登录
    do_login(user).await?;
    let header = get_download_header(&id_str, user);
    let data = response_for_download(&url, header).await?;
    if data.status() == StatusCode::OK {
        let p: Bytes = data.bytes().await?;
        return Ok(p);
    }
    // 登录失败抛出错误
    Err(Box::new(OsuMapDownloadError::new("下载失败")))
}
