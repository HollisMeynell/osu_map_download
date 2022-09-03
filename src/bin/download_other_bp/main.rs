use std::fmt::format;
use anyhow::*;
use osurs_map_download::prelude::*;
use regex::Regex;

const REG_PATH: &str = "SOFTWARE\\Classes\\osu\\DefaultIcon";

fn get_osu_path() -> Result<String>{
    let hk = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let (tk, rd) = hk.create_subkey(REG_PATH)?;
    let path = tk.get_raw_value("")?;
    Ok(path.to_string())
}

#[derive(Clone, Copy)]
enum Mode {
    Osu,
    Taiko,
    Catch,
    Mania,
}

impl Mode {
    fn to_str(&self) -> &str {
        match osu_mode {
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

async fn get_sid(uid: i64, osu_mode: Mode, index: i32) -> Result<i64> {
    let mode_str = osu_mode.to_str();
    let res = reqwest::get(format!("https://osu.ppy.sh/users/{uid}/scores/best?mode={mode_str}&limit=1&offset={index}")).await?;
    let data = res.text().await?;

    let reg = Regex::new(r#"(?x)"beatmapset_id":(?P<sid>\d+)"#).unwrap();
    let data = reg
        .captures(&data)
        .and_then(|cap| cap.name("sid"))
        .map(|d| d.as_str())
        .unwrap();
    Ok(data.parse()?)
}

fn check_username(name: &str) -> bool {
    let regex = Regex::new(r#"^[\w _\-\[\]]+$"#).unwrap();
    regex.captures(name).is_some()
}

fn get_username(text: &str) -> String {
    println!(text);
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

#[tokio::main]
async fn main() -> Result<()> {
    let username = get_username("请输入你自己的 osu 用户名: ");
    let password = rpassword::prompt_password(format!("请输入 {username} 的密码: "))?;
    println!("登陆中...");
    let user = UserSession::new(username, &password).await?;

    let other_name = get_username("输入获取谁的bp:");
    println!("请输入模式数字(0:osu 1:taiko 2:catch 3:mania):");
    let mut osu_mode = String::new();
    std::io::stdin().read_line(&mut osu_mode).expect("输入错误");
    let mode = Mode::press(&osu_mode);
    let mut bid_list = vec![];
    for index in 0..100 {
        bid_list.push(get_sid(sid, mode, index).await?);
    }
    Ok(())
}

///
/// get peppy's Best Performance (I believe it will not change ...)
#[tokio::test]
async fn test_get_bp_sid() -> Result<()> {
    let mut sid = get_sid(2, Mode::Osu, 0).await?;
    assert_eq!(3720, sid);
    sid = get_sid(2, Mode::Taiko, 0).await?;
    assert_eq!(380864, sid);
    sid = get_sid(2, Mode::Catch, 0).await?;
    assert_eq!(118, sid);
    sid = get_sid(2, Mode::Mania, 0).await?;
    assert_eq!(63089, sid);

    Ok(())
}

#[test]
fn test_name_regex() {
    assert_eq!(false, check_username("( name"));
    assert_eq!(true, check_username("pe_ppy[fuck]"));
    assert_eq!(true, check_username("-Spring Night-"));
}
