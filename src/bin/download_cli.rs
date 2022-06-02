use anyhow::{anyhow, Context, Result};
use clap::Parser;
use directories::BaseDirs;
use osu_map_download::util::{do_home, do_login, download, OsuMapDownloadError, UserSession};
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
    #[clap(short, help = "清空缓存文件")]
    clear: bool,
    #[clap(short, long, help = "保存路径,默认程序所在位置")]
    save_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    username: String,
    password: String,
    session: String,
    save_path: String,
}

async fn run(sid:u64, user:&mut UserSession, path:&PathBuf) -> Result<()> {
    if !path.is_dir() {
        return Err(anyhow!("\"{:?}\"路径不存在", path));
    }
    println!("正在下载...");
    download(
        sid,
        user,
        path.join(&format!(r"{}.osz", sid)).as_path(),
    )
    .await?;

    println!("下载完成");
    Ok(())
}
/// windows:$HOME\AppData\Roaming\OsuMapDownloader\config.json
/// linux:$HOME/.config/OsuMapDownloader\config.json
/// macos:$HOME/Library/Application Support/OsuMapDownloader\config.json
fn get_config_path() -> Result<PathBuf> {
    let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;

    let dir = basedir
        .config_dir()
        .join("OsuMapDownloader")
        .join("config.json");
    // 当前路径
    // let mut dir = env::current_dir()?;
    // dir.push("config.json");

    let file = dir.as_path();

    if !file.is_file() {
        let file = fs::File::create(file);
        if file.is_err() {
           return Err(anyhow!("创建配置文件失败,请检查程序目录下是否有名为'config.json'的文件夹"));
        }
    }
    return Ok(dir)
}

fn read_config(path:&Path) -> Result<(UserSession, PathBuf)>{

    let config = fs::read(path).with_context(|| "读取用户配置失败")?;
    let config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行")?;
    let mut user = UserSession::new(&config.username,&config.password);
    user.read_session(&config.session);
    let path = PathBuf::from(config.save_path);
    Ok((user, path))
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
fn save_download_path(path:String, user_config_path: &Path) -> Result<()> {
    let dir = fs::read_link(&Path)?;
    if !dir.is_dir() {
        println!("{:?}文件夹不存在,正在创建...", dir);
        fs::create_dir_all(dir.as_path());
    }
    let config = fs::read(user_config_path).with_context(|| "读取用户配置失败")?;
    let mut config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行")?;
    config.save_path = path.to_string();
    let config_str =
        serde_json::to_string(&config)?;
    fs::write(user_config_path, config_str.as_bytes())?;
    println!("设置完毕!");
    Ok(())
}

async fn login_no_name() -> Result<()>{
    println!("enter osu name:");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();
    let password = rpassword::prompt_password(format!("enter {username}'s password:\n"))?;
    let mut user = UserSession::new(&username, &password);
    println!("login. . .");
    do_home(&mut user).await?;
    do_login(&mut user).await?;
    println!("success!");

    let config = Config{
        username,
        password,
        session:user.save_session(),
        save_path:"./".to_string()
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
    do_home(&mut user).await?;
    do_login(&mut user).await?;
    println!("success!");

    let config = Config{
        username: username.clone(),
        password,
        session:user.save_session(),
        save_path:"./".to_string()
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
        let mut  path = get_config_path()?;
        // 清除配置文件
        fs::remove_file(path.as_path());
        // 移除目录
        if path.pop() {
            fs::remove_dir(path.as_path());
        }
        println!("清理完毕!");
    }
    if cli.login {
       if let Some(s) = cli.user {
           login_name(&s).await?;
       } else {
           login_no_name().await?;
       }
        return Ok(());
    }
    if let Some(path) = cli.save_path {
        let confih_path = get_config_path()?;
        save_download_path(
            path,
            confih_path.as_path()
        );
        return Ok(());
    }
    if let Some(sid) = cli.sid {
        let path = get_config_path()?;
        let(mut user, save_path) = read_config(path.as_path())?;
        run(sid, &mut user, &save_path).await?;
        save_user_cookie(&mut user, path.as_path());
    }
    Ok(())
}
