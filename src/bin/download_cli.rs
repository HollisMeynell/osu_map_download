use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use osu_map_download::util::{do_home, do_login, download, UserSession};

#[derive(Debug, Parser)]
#[clap(name = "osu beatmap downloader")]
#[clap(author = "-Sprint Night-, CookieBacon")]
#[clap(version = "0.1")]
#[clap(about = "A cli to help your download osu beatmap")]
struct Cli {
    #[clap(help = "输入下载谱面的sid")]
    sid: Option<u64>,
    #[clap(short, help = "登录选项")]
    login: bool,
    #[clap(short, long, help = "用户名", allow_hyphen_values = true)]
    user: Option<String>,
    #[clap(short, help = "清空缓存文件")]
    clear: bool,
    #[clap(short, long, help = "保存路径,默认程序所在位置")]
    save_path: Option<String>,
    #[clap(short, help = "下载包含视频的文件,默认不包含")]
    video: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    save_path: String,
}

async fn run(sid: u64, user: &mut UserSession, path: &PathBuf, video: bool) -> Result<()> {
    if !path.is_dir() {
        return Err(anyhow!("\"{:?}\"路径不存在", path));
    }
    println!("正在下载...");

    let d_path = path.join(&format!(r"{}.osz", sid));
    download(sid, user, &d_path, video).await?;

    println!("下载完成");
    Ok(())
}

/// windows:$HOME\AppData\Roaming\OsuMapDownloader\config.json
/// linux:$HOME/.config/OsuMapDownloader\config.json
/// macos:$HOME/Library/Application Support/OsuMapDownloader\config.json
fn get_config_path() -> Result<PathBuf> {
    let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;

    let dir = basedir.config_dir().join("OsuMapDownloader");
    // 当前路径
    // let mut dir = env::current_dir()?;
    // dir.push("config.json");

    if !dir.is_dir() {
        fs::create_dir_all(dir.as_path())?;
    }

    let dir = dir.join("config.json");

    let file = dir.as_path();

    if !file.is_file() {
        let file = fs::File::create(file);
        if let Err(e) = file {
            return Err(anyhow!("创建配置文件失败:\n{e}"));
        }
    }

    Ok(dir)
}

fn read_config(path: &Path) -> Result<PathBuf> {
    let config = fs::read(path).with_context(|| "读取用户配置失败")?;
    let config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败,请使用'-c'参数重置配置后重新运行")?;
    let path = PathBuf::from(config.save_path);
    Ok(path)
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

async fn login_no_name() -> Result<()> {
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

    let config = Config {
        save_path: "./".to_string(),
    };
    let config_str = serde_json::to_string(&config)?;
    let file = get_config_path()?;
    fs::write(file.as_path(), config_str.as_bytes())?;

    user.save()?;
    Ok(())
}

async fn login_name(username: &String) -> Result<()> {
    let password = rpassword::prompt_password(format!("enter {username}'s password:\n"))?;
    println!("login. . .");
    let mut user = UserSession::new(username, &password);
    do_home(&mut user).await?;
    do_login(&mut user).await?;
    println!("success!");

    let config = Config {
        save_path: "./".to_string(),
    };
    let config_str = serde_json::to_string(&config)?;
    let file = get_config_path()?;
    fs::write(file.as_path(), config_str.as_bytes())?;

    user.save()?;
    Ok(())
}

async fn handle_login(cli: &Cli) -> Result<()> {
    if let Some(s) = &cli.user {
        login_name(s).await?;
    } else {
        login_no_name().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    if cli.clear {
        let mut path = get_config_path()?;
        // 清除配置文件
        fs::remove_file(path.as_path())?;
        // 移除目录
        if path.pop() {
            fs::remove_dir(path.as_path())?;
        }
        println!("清理完毕!");
    }

    if cli.login {
        handle_login(&cli).await?;
    }
    if let Some(path) = cli.save_path {
        let confih_path = get_config_path()?;
        save_download_path(path, confih_path.as_path())?;
    }
    if let Some(sid) = cli.sid {
        let path = get_config_path()?;
        let user = UserSession::from();
        if user.is_err() {
            return Err(anyhow!("没有登录,请使用'-l [-u <username>]'登录后使用"));
        }
        let mut user = user.unwrap();
        let save_path = read_config(path.as_path())?;
        run(sid, &mut user, &save_path, cli.video).await?;
        user.save()?;
    }
    Ok(())
}
