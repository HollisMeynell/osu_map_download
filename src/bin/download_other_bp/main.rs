use std::fmt::format;
use std::path::{Path, PathBuf};
use anyhow::*;
use osurs_map_download::prelude::*;
use regex::Regex;
use osu_db::collection::*;

const REG_PATH: &str = "SOFTWARE\\Classes\\osu\\DefaultIcon";

// fn get_osu_path() -> Result<String>{
//     let hk = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
//     let (tk, rd) = hk.create_subkey(REG_PATH)?;
//     let path = tk.get_raw_value("")?;
//     Ok(path.to_string())
// }

#[derive(Clone, Copy)]
enum Mode {
    Osu,
    Taiko,
    Catch,
    Mania,
}

impl Mode {
    fn to_str(&self) -> &str {
        match self {
            Mode::Osu => { "osu" }
            Mode::Taiko => { "taiko" }
            Mode::Catch => { "fruits" }
            Mode::Mania => { "mania" }
        }
    }
    fn press(i: &str) -> Self {
        let i = i.trim();
        match i {
            "0" => { Mode::Osu }
            "1" => { Mode::Taiko }
            "2" => { Mode::Catch }
            "3" => { Mode::Mania }
            _ => { Mode::Osu }
        }
    }
}

async fn get_sid(uid: i64, osu_mode: Mode, index: i32) -> Result<(String, String)> {
    let mode_str = osu_mode.to_str();
    let res = reqwest::get(format!("https://osu.ppy.sh/users/{uid}/scores/best?mode={mode_str}&limit=1&offset={index}")).await?;
    let data = res.text().await?;

    let reg = Regex::new(r#"(?x)"beatmapset_id":(?P<sid>\d+)"#)?;
    let sid = reg
        .captures(&data)
        .and_then(|cap| cap.name("sid"))
        .map(|d| d.as_str())
        .unwrap();
    let reg = Regex::new(r#"(?x)"checksum":"(?P<sid>\w+)""#)?;
    let data = reg
        .captures(&data)
        .and_then(|cap| cap.name("sid"))
        .map(|d| d.as_str())
        .unwrap();
    Ok((String::from(sid),String::from(data)))
}

async fn get_osu_id(name:&str) -> Result<i64> {
    let get = reqwest::get(format!("https://osu.ppy.sh/users/{name}")).await?;
    let url = get.url().to_string();
    if let Some(mut d) = url.rfind("/") {
        d = d + 1;
        let uid = &url[d..];
        println!("{}", uid);
        let out = uid.parse::<i64>()?;
        return Ok(out);
    }
    return Err(Error::msg("not found"));
}

fn check_username(name: &str) -> bool {
    let regex = Regex::new(r#"^[\w _\-\[\]]+$"#).unwrap();
    regex.captures(name).is_some()
}

fn get_username(text: &str) -> String {
    println!("{}", text);
    let mut buffer = String::new();
    loop {
        std::io::stdin()
            .read_line(&mut buffer)
            .expect("非法的用户名输入，请重试");
        let name = buffer.trim().to_string();
        if check_username(&name) {
            return name;
        }
        println!("用户名校验未通过,请确认是否正确");
        buffer.clear();
    }
}

fn get_osu_path() -> String {
    String::from("")
}

#[tokio::main]
async fn main() -> Result<()> {
    let username = get_username("请输入你自己的 osu 用户名: ");
    let password = rpassword::prompt_password(format!("请输入 {username} 的密码: "))?;
    println!("登陆中...");
    let mut user = UserSession::new(username, &password).await?;

    let other_name = get_username("输入获取谁的bp:");
    let uid = get_osu_id(&other_name).await?;
    println!("请输入模式数字(0:osu 1:taiko 2:catch 3:mania):");
    let mut osu_mode = String::new();
    std::io::stdin().read_line(&mut osu_mode).expect("输入错误");
    let mode = Mode::press(&osu_mode);

    let path = get_osu_path();
    if path.is_err() {
        return Error(path.expect_err("未找到osu安装路径"));
    }
    let mut path = PathBuf::from(path.unwrap());
    path.push("songs");
    let mut beatmap_hashes = vec![];
    for index in 0..100 {
        let (sid, md5) = get_sid(uid, mode, index).await?;
        download(&[sid], &mut user, path.as_path(), false).await?;
        beatmap_hashes.push(Some(md5));
    }
    path.pop();


    Ok(())
}

#[tokio::test]
async fn test_get_osu_id() {
    let id = get_osu_id("-Spring Night-").await.unwrap();
    assert_eq!(17064371, id);
}

/// get peppy's Best Performance (I believe it will not change ...)
///
#[tokio::test]
async fn test_get_bp_sid() -> Result<()> {
    let mut sid = get_sid(2, Mode::Osu, 0).await?;
    assert_eq!("3720", sid);
    sid = get_sid(2, Mode::Taiko, 0).await?;
    assert_eq!("380864", sid);
    sid = get_sid(2, Mode::Catch, 0).await?;
    assert_eq!("118", sid);
    sid = get_sid(2, Mode::Mania, 0).await?;
    assert_eq!("63089", sid);

    Ok(())
}

#[test]
fn test_name_regex() {
    assert_eq!(false, check_username("( name"));
    assert_eq!(true, check_username("pe_ppy[fuck]"));
    assert_eq!(true, check_username("-Spring Night-"));
}
