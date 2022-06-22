use crate::client;
use crate::error::OsuMapDownloadError;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header::{HeaderMap, CONTENT_TYPE, COOKIE};
use std::collections::HashMap;

lazy_static! {
    static ref REG_XSRF: Regex = Regex::new(r"XSRF-TOKEN=([\w]+);").unwrap();
    static ref REG_COOKIE: Regex = Regex::new(r"osu_session=([\w%]+);").unwrap();
}

const HOME_PAGE_URL: &str = "https://osu.ppy.sh/home";
static LOGIN_URL: &str = "https://osu.ppy.sh/session";

/// 用户信息记录,包含密码,登录后的session
/// 包含的session信息可重用,请重用此结构
/// 可以将session保存出来
#[derive(Debug, Default, PartialEq)]
pub struct UserSession {
    name: String,
    password: String,
    token: String,
    session: String,
}

/// 生成请求用到的cookie字符串
#[inline]
fn new_cookie(xsrf: &str, cookie: &str) -> String {
    format!("XSRF-TOKEN={xsrf}; osu_session={cookie};")
}

impl UserSession {
    /// 通过账号密码生产记录
    pub async fn new<T: Into<String>, U: Into<String>>(username: T, password: U) -> Result<Self> {
        let mut session = UserSession {
            name: username.into(),
            password: password.into(),
            token: String::new(),
            session: String::new(),
        };

        session.refresh().await?;

        Ok(session)
    }

    /// Try login into osu. Return error if any network or account error occur
    pub async fn refresh(&mut self) -> Result<()> {
        self.update_access().await?;
        self.login().await?;
        Ok(())
    }

    /// Get token and session cookie. This should be called before login cuz login needs
    /// those value.
    async fn update_access(&mut self) -> Result<()> {
        let mut header = HeaderMap::new();
        header.insert(
            "cookie",
            new_cookie(&self.token, &self.session).parse().unwrap(),
        );
        let response = client::get(HOME_PAGE_URL, header)
            .await
            .with_context(|| "请求主页失败")?;

        match response.status() {
            reqwest::StatusCode::OK => {
                self.update(response.headers());
                Ok(())
            }
            reqwest::StatusCode::BAD_REQUEST => {
                Err(OsuMapDownloadError::IncorrectPasswordError.into())
            }
            _ => Err(OsuMapDownloadError::Unknown.into()),
        }
    }

    pub fn new_header(&self, back_url: &str) -> HeaderMap {
        let mut header = HeaderMap::new();
        header.insert(
            COOKIE,
            new_cookie(&self.token, &self.session).parse().unwrap(),
        );

        header.insert("referer", back_url.parse().unwrap());
        header.insert(
            CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        header
    }

    /// Try login with current data
    async fn login(&mut self) -> Result<()> {
        let mut header = HeaderMap::new();
        header.insert("referer", HOME_PAGE_URL.parse()?);
        header.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        header.insert(
            reqwest::header::COOKIE,
            new_cookie(&self.token, &self.session).parse()?,
        );

        let mut body = HashMap::new();
        body.insert("_token".to_string(), &self.token);
        body.insert("username".to_string(), &self.name);
        body.insert("password".to_string(), &self.password);

        let response = client::post(LOGIN_URL, header, &body)
            .await
            .with_context(|| "登录请求无回复")?;

        match response.status() {
            reqwest::StatusCode::OK => {
                self.update(response.headers());
                Ok(())
            }
            reqwest::StatusCode::FORBIDDEN => {
                Err(OsuMapDownloadError::IncorrectPasswordError.into())
            }
            _ => {
                println!("status:{}", response.status());
                Err(OsuMapDownloadError::Unknown.into())
            }
        }
    }

    /// 将当前的 cookie 和 token 信息转换成可供保存的字符串。
    pub fn to_recoverable(&self) -> String {
        format!("{},{}", self.token, self.session)
    }

    /// 通过保存的session数据恢复
    pub fn from_recoverable(username: &str, data: &str) -> Option<UserSession> {
        let parts: Vec<&str> = data.split(',').collect();
        if parts.len() < 2 {
            return None;
        }
        Some(UserSession {
            name: username.to_string(),
            password: String::new(),
            token: parts[0].to_string(),
            session: parts[1].to_string(),
        })
    }

    /// Get immutable reference to inner name
    pub fn username(&self) -> &str {
        &self.name
    }

    // 更新 token 和 session。如果传入的 HeaderMap 没有满足更新的值，旧的值会保留
    pub fn update(&mut self, header_map: &HeaderMap) {
        let all_headers = header_map.get_all("set-cookie");
        let mut xsrf_change = false;
        let mut cookie_change = false;
        for header in all_headers {
            // early return to save regexp match time
            if xsrf_change && cookie_change {
                return;
            }

            let str = header.to_str();
            if str.is_err() {
                continue;
            }
            // it is safe to unwrap now
            let str = str.unwrap();

            // FIXME: 这里的元组只是一个暂时的解决地狱 if 嵌套的方案，等 Rust 1.63 版本发布之后，
            // 可以使用 if let chain 来重写这个条件判断
            // 链接：https://github.com/rust-lang/rust/pull/94927
            if let (true, Some(xsrf)) = (!xsrf_change, REG_XSRF.captures(str)) {
                let old_token = &self.token;
                self.token = xsrf
                    .get(1)
                    .map_or_else(|| old_token.clone(), |v| v.as_str().to_string());
                xsrf_change = true;

                continue;
            }

            if let (true, Some(cookie_match)) = (!cookie_change, REG_COOKIE.captures(str)) {
                let old_session = &self.session;
                self.session = cookie_match
                    .get(1)
                    .map_or_else(|| old_session.clone(), |v| v.as_str().to_string());
                cookie_change = true;
            }
        }
    }
}

#[test]
fn test_user_from_recoverable() {
    // test valid
    let user = UserSession::from_recoverable("abc", "def,123");
    assert_eq!(
        user,
        Some(UserSession {
            name: String::from("abc"),
            password: String::new(),
            token: String::from("def"),
            session: String::from("123"),
        })
    );
    let user = user.unwrap();
    assert_eq!(user.to_recoverable(), "def,123");

    // test invalid
    let user = UserSession::from_recoverable("abc", "def");
    assert_eq!(user, None);
}

#[test]
fn test_user_update_header() {
    let mut user = UserSession::from_recoverable("foo", "bar,123").unwrap();
    let mut headers = HeaderMap::new();
    use reqwest::header::HeaderValue;
    headers.insert(
        "set-cookie",
        HeaderValue::from_str("XSRF-TOKEN=abcdef12345;").unwrap(),
    );
    headers.append(
        "set-cookie",
        HeaderValue::from_str("osu_session=ghijklm78901;").unwrap(),
    );

    user.update(&headers);
    assert_eq!(
        user,
        UserSession {
            name: String::from("foo"),
            password: String::new(),
            session: String::from("ghijklm78901"),
            token: String::from("abcdef12345"),
        }
    )
}

#[test]
fn test_user_new_header() {
    let user = UserSession::from_recoverable("foo", "bar,123").unwrap();
    let header = user.new_header("12345");
    let mut expect = HeaderMap::new();
    expect.insert(COOKIE, new_cookie("bar", "123").parse().unwrap());
    expect.insert(
        "referer",
        format!("https://osu.ppy.sh/beatmapsets/12345")
            .parse()
            .unwrap(),
    );
    expect.insert(
        CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    assert_eq!(header, expect);
}
