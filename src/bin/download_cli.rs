use anyhow::{anyhow, Context, Result};
use clap::Parser;
use directories::BaseDirs;
use osu_map_download::util::{do_login, download, OsuMapDownloadError, UserSession};
use serde::{Deserialize, Serialize};
use std::{env, fs};
use std::io::Write;
use std::path::{Path, PathBuf};
use reqwest::StatusCode;

#[derive(Debug, Parser)]
#[clap(name = "osu beatmap downloader")]
#[clap(author = "-Sprint Night-, CookieBacon")]
#[clap(version = "0.1")]
#[clap(about = "A cli to help your download osu beatmap")]
struct Cli {
    #[clap(help = "输入下载谱面的sid")]
    sid: Option<u64>,
    #[clap(short, help = "登录")]
    login: bool,
    #[clap(short, long, help = "用户名")]
    user: Option<String>,
    #[clap(short)]
    clear: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    username: String,
    password: String,
    session: String,
}

async fn run(sid:u64, user:&mut UserSession) -> Result<()> {


    println!("正在下载...");

    download(
        sid,
        user,
        Path::new(&format!(r".\{}.zip", sid)),
    )
    .await?;

    println!("下载完成");
    Ok(())
}

fn get_config_path() -> Result<PathBuf> {
    let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;
    // $HOME\AppData\Roaming\OsuMapDownloader\config.json
    let dir = basedir
        .config_dir()
        .join("OsuMapDownloader")
        .join("config.json");
    // 本地路径
    // let mut dir = env::current_dir()?;
    // dir.push("config.json");

    let file = dir.as_path();

    if !file.is_file() {
        let file = fs::File::create(file);
        if file.is_err() {
           anyhow!("创建配置文件失败,请检查程序目录下是否有名为'config.json'的文件夹");
        }
    }
    return Ok(dir)
}

fn read_config(path:&Path) -> Result<UserSession>{

    let config = fs::read(path).with_context(|| "读取用户配置失败")?;
    let config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行")?;
    let mut user = UserSession::new(&config.username,&config.password);
    user.read_session(&config.session);

    Ok(user)
}

fn save_user_cookie(user: &mut UserSession, user_config_path: &Path) -> Result<()> {
    let config = fs::read(user_config_path).with_context(|| "读取用户配置失败")?;
    let mut  config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行")?;
    config.session = user.save_session();
    let config_str =
        serde_json::to_string(&config)?;
    fs::write(user_config_path, config_str.as_bytes())?;
    Ok(())
}
async fn login_no_name() -> Result<()>{
    println!("enter osu name:");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username)?;
    let password = rpassword::prompt_password(format!("enter {username}'s password:\n"))?;
    let mut user = UserSession::new(&username, &password);
    println!("login. . .");
    do_login(&mut user).await?;
    println!("success!");

    let config = Config{
        username,
        password,
        session:user.save_session(),
    };
    let config_str =
        serde_json::to_string(&config)?;
    let file = get_config_path()?;
    fs::write(file.as_path(), config_str.as_bytes())?;

    Ok(())
}
async fn login_name(username: &String) -> Result<()>{
    let password = rpassword::prompt_password(format!("enter {username}'s password:\n"))?;
    println!("login. . .");
    let mut user = UserSession::new(username, &password);
    do_login(&mut user).await?;
    println!("success!");

    let config = Config{
        username: username.clone(),
        password,
        session:user.save_session(),
    };
    let config_str =
        serde_json::to_string(&config)?;
    let file = get_config_path()?;
    fs::write(file.as_path(), config_str.as_bytes())?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli:Cli = Cli::parse();
    if cli.clear {
        let path = get_config_path()?;
        fs::remove_file(path.as_path());
    }
    if cli.login {
       if let Some(s) = cli.user {
           login_name(&s).await?;
       } else {
           login_no_name().await?;
       }
    } else {
        if let Some(sid) = cli.sid {
            let path = get_config_path()?;
            let mut user = read_config(path.as_path())?;
            run(sid, &mut user).await?;
            save_user_cookie(&mut user, path.as_path());
        }
    }
    Ok(())
}
