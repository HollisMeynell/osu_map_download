use regex::Regex;
use reqwest::header::HeaderMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref REG_XSRF: Regex = Regex::new(r"XSRF-TOKEN=([\w\d]+);").unwrap();
    static ref REG_COOKIE: Regex = Regex::new(r"osu_session=([\w\d%]+);").unwrap();
}

/// 用户信息记录,包含密码,登录后的session
/// 包含的session信息可重用,请重用此结构
/// 可以将session保存出来
pub struct UserSession {
    pub name: String,
    pub password: String,
    pub token: String,
    pub session: String,
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
            if let Some(m) = s.get(1) {
                self.token = m.as_str().to_string();
            }
            if let Some(m) = s.get(2) {
                self.session = m.as_str().to_string();
            }
        }
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

