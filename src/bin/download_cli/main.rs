/// Enable pswd-store features to store user password.
#[cfg(feature = "pswd-store")]
mod pswd_store;

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
    #[clap(short, help = "进入登录模式，只更新 cookie 信息，不下载歌曲")]
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

/// Data for storing user's username, reusable cookie data and default download path.
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    username: String,
    download_path: String,
}

async fn run(
    sid: Vec<String>,
    user: &mut UserSession,
    path: &PathBuf,
    no_video: bool,
) -> Result<()> {
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
fn find_or_new_cfg_path() -> Result<PathBuf> {
    let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;

    let dir = basedir.config_dir().join("OsuMapDownloader");

    if !dir.is_dir() {
        println!("找不到配置文件目录，正在新建...");
        fs::create_dir_all(dir.as_path()).with_context(|| "无法创建配置文件目录")?;
    }

    let config_path = dir.join("config.json");

    if !config_path.is_file() {
        println!("找不到配置文件，正在新建...");
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

fn save_config(user: &UserSession, download_path: &Path) -> Result<()> {
    let config = Config {
        username: user.username().to_string(),
        download_path: download_path.to_str().expect("非法的保存路径").to_string(),
    };
    let config_str = serde_json::to_string(&config)?;
    let config_path = find_or_new_cfg_path()?;
    fs::write(config_path, config_str.as_bytes()).with_context(|| "写入配置文件时出错")?;
    Ok(())
}

// save recoverable data into cache directory
fn save_cookie(user: &UserSession) -> Result<()> {
    let basedir = BaseDirs::new().unwrap();
    let cache_dir = basedir.cache_dir();

    let cache_dir = cache_dir.join("osu-map-downloader");
    if !cache_dir.is_dir() {
        fs::create_dir(&cache_dir).with_context(|| "创建缓存文件夹时出错")?;
    }

    let cache_file = cache_dir.join("user-session");
    fs::write(cache_file, user.to_recoverable()).with_context(|| "写入用户缓存时出错")?;

    Ok(())
}

// get session from cache directory
fn load_cookie() -> Option<String> {
    let basedir = BaseDirs::new().unwrap();
    let cache_dir = basedir.cache_dir();
    let cache_dir = cache_dir.join("osu-map-downloader");
    if !cache_dir.is_dir() {
        return None;
    }
    let cache_file = cache_dir.join("user-session");
    fs::read_to_string(cache_file).ok()
}

// Do rm -rf for $CACHE_DIR/osu_map_download/
fn clean_cookie() -> Result<()> {
    let basedir = BaseDirs::new().unwrap();
    let cache_dir = basedir.cache_dir();
    let cache_dir = cache_dir.join("osu-map-downloader");
    if !cache_dir.is_dir() {
        return Ok(());
    }

    Ok(fs::remove_dir_all(cache_dir)?)
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
    config.download_path = path;
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

    let config_path = find_or_new_cfg_path()?;

    if cli.clear {
        let mut config_path = config_path.clone();
        // 清除配置文件
        fs::remove_file(&config_path)?;
        // 移除目录
        if config_path.pop() {
            fs::remove_dir(config_path)?;
        }

        // clean sessions
        clean_cookie()?;
        println!("清理完毕!");
        return Ok(());
    }

    if cli.login {
        let user = try_login(cli.user).await?;
        // FIXME: we should use download path here
        save_cookie(&user)?;
        return Ok(());
    }

    if cli.sid.is_empty() {
        anyhow::bail!("请指定谱面 sid，使用 -h 选项来获取更多信息")
    }

    let config = read_config(&config_path);

    let recover_data = load_cookie();
    // if no previous session, handle login
    if recover_data.is_none() {
        let (mut session, download_to) = match config {
            // if config is valid, we recover the user session from previous data
            Ok(cfg) => (
                try_login(Some(cfg.username)).await?,
                Path::new(&cfg.download_path).to_path_buf(),
            ),
            // if config is invalid, try to prompt up a login process for user
            Err(_) => (try_login(None).await?, PathBuf::new()),
        };

        run(cli.sid, &mut session, &download_to, cli.video).await?;
        save_cookie(&session)?;
        return Ok(());
    }

    let recover_data = recover_data.unwrap();
    let (mut session, download_to) = match config {
        // if config is valid, we recover the user session from previous data
        Ok(cfg) => (
            UserSession::from_recoverable(&cfg.username, &recover_data)
                .ok_or_else(|| anyhow::anyhow!("非法的 session 数据，请使用 -c 参数清理重试"))?,
            Path::new(&cfg.download_path).to_path_buf(),
        ),
        // if config is invalid, try to prompt up a login process for user
        Err(_) => (try_login(None).await?, PathBuf::new()),
    };

    run(cli.sid, &mut session, &download_to, cli.video).await?;
    save_cookie(&session)?;

    if let Some(path) = cli.save_path {
        save_download_path(path, config_path.as_path())?;
    }

    Ok(())
}
