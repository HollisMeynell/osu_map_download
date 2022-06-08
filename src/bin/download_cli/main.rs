mod pswd;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use osu_map_download::prelude::*;

#[derive(Debug, Parser)]
#[clap(name = "osu beatmap downloader")]
#[clap(author = "[-Spring Night-, CookieBacon]")]
#[clap(version = "0.1")]
#[clap(about = "A cli to help your download osu beatmap")]
struct Cli {
    #[clap(help = "输入下载谱面的sid，可以用空格隔开输入多个")]
    sid: Vec<String>,
    #[clap(short, help = "进入登录模式")]
    login: bool,
    #[clap(short, long, help = "用户名", allow_hyphen_values = true)]
    user: Option<String>,
    #[clap(short, help = "清空缓存文件")]
    clear: bool,
    #[clap(short, long, help = "保存路径，默认当前目录")]
    save_path: Option<String>,
    #[clap(short, help = "不下载包含视频的文件，默认不下载视频")]
    video: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    session: String,
    save_path: String,
}

async fn run(sid: Vec<String>, user: &mut UserSession, path: &PathBuf, no_video: bool) -> Result<()> {
    if !path.is_dir() {
        return Err(anyhow!("\"{:?}\"路径不存在", path));
    }
    println!("正在下载...");
    download(&sid, user, path.as_path(), no_video).await?;

    println!("下载完成");
    Ok(())
}

/// Return configuration path for this application.
/// If configuration file doesn't exist, it will try to create them.
///
/// Default configuration path in different OS:
/// windows:$HOME\AppData\Roaming\OsuMapDownloader\config.json
/// linux:$HOME/.config/OsuMapDownloader\config.json
/// macos:$HOME/Library/Application Support/OsuMapDownloader\config.json
fn get_or_new_config_path() -> Result<PathBuf> {
    let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;

    let dir = basedir.config_dir().join("OsuMapDownloader");

    if !dir.is_dir() {
        fs::create_dir_all(dir.as_path())?;
    }

    let config_path = dir.join("config.json");

    if !config_path.is_file() {
        fs::File::create(config_path.as_path()).with_context(|| "无法创建配置文件")?;
    }

    Ok(config_path)
}

fn read_config(path: &Path) -> Result<Config> {
    let config = fs::read(path).with_context(|| "读取用户配置失败")?;
    let config: Config = serde_json::from_slice(&config).with_context(|| {
        "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行"
    })?;
    Ok(config)
}

fn save_user_cookie(user: &mut UserSession, user_config_path: &Path) -> Result<()> {
    let config = fs::read(user_config_path).with_context(|| "读取用户配置失败")?;
    let mut config: Config = serde_json::from_slice(&config).with_context(|| {
        "解析用户配置失败,请使用'-l'参数登录,或者请加'-c'参数重置配置后重新运行"
    })?;
    config.session = user.to_recoverable();
    let config_str = serde_json::to_string(&config)?;
    fs::write(user_config_path, config_str.as_bytes())?;
    Ok(())
}

fn save_download_path(path: String, user_config_path: &Path) -> Result<()> {
    let mut dir = fs::canonicalize(&path);
    if dir.is_err() {
        println!("{:?}文件夹不存在,正在创建...", &path);
        fs::create_dir_all(&path)?;
        dir = fs::canonicalize(&path);
    }
    let dir = dir.unwrap();
    if !dir.is_dir() {
        return Err(anyhow!("无法使用该路径!"));
    }
    let config = fs::read(user_config_path).with_context(|| "读取用户配置失败")?;
    let mut config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-c'参数重置配置后重新运行")?;
    config.save_path = path;
    let config_str = serde_json::to_string(&config)?;
    fs::write(user_config_path, config_str.as_bytes())?;
    println!("设置完毕!");
    Ok(())
}

async fn try_login(mut username: Option<String>) -> Result<UserSession> {
    if username.is_none() {
        println!("输入你的 osu 用户名: ");
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        let buffer = buffer.trim().to_string();
        username.replace(buffer);
    }

    // we have handle None case at above, it is safe to invoke unwrap here
    let username = username.unwrap();
    let password = rpassword::prompt_password(format!("请输入 {username} 的密码: "))?;

    UserSession::new(&username, &password).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    if cli.clear {
        let mut path = get_or_new_config_path()?;
        // 清除配置文件
        fs::remove_file(path.as_path())?;
        // 移除目录
        if path.pop() {
            fs::remove_dir(path.as_path())?;
        }
        println!("清理完毕!");
    }

    if cli.login {
        try_login(cli.user).await?;
        return Ok(());
    }

    if let Some(path) = cli.save_path {
        let config_path = get_or_new_config_path()?;
        save_download_path(path, config_path.as_path())?;
        return Ok(());
    }

    if cli.sid.is_empty() {
        anyhow::bail!("请指定谱面 sid，使用 -h 选项来获取更多信息")
    }

    let config_path = get_or_new_config_path()?;
    let config = read_config(config_path.as_path());
    let (mut user, save_to) = if let Ok(config) = config {
        (
            UserSession::new(&config.username, &config.password).await?,
            Path::new(&config.save_path).to_path_buf(),
        )
    } else {
        (try_login(None).await?, PathBuf::new())
    };
    run(cli.sid, &mut user, &save_to, cli.video).await?;
    save_user_cookie(&mut user, config_path.as_path())?;
    Ok(())
}
