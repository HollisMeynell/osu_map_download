use anyhow::{anyhow, Context, Result};
use clap::Parser;
use directories::BaseDirs;
use osu_map_download::util::{download, OsuMapDownloadError, UserSession};
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
    #[clap(short, long,)]
    login: bool,
    #[clap(short, long, help = "用户名")]
    user: Option<String>,

}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    username: String,
    password: String,
    session: String,
}

async fn run() -> Result<()> {
    let cli:Cli = Cli::parse();

    let mut username = cli.username;
    let mut password = cli.password;

    if username.is_none() || password.is_none() {
        println!("正在从配置文件中读取用户信息......");
        let basedir = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("找不到你的系统配置目录"))?;
        // C:\Users\ABCDE\AppData\Roaming\OsuMapDownloader\config.json
        let path = basedir
            .config_dir()
            .join("OsuMapDownloader")
            .join("config.json");
        let config = fs::read(path).with_context(|| "读取用户配置失败")?;
        let config: Config = serde_json::from_slice(&config)
            .with_context(|| "解析用户配置失败，请删除掉配置文件重试！")?;
        username.replace(config.username);
        password.replace(config.password);
    }

    let mut user = UserSession::new(&username.unwrap(), &password.unwrap());

    println!("正在下载...");

    download(
        cli.sid.unwrap(),
        &mut user,
        Path::new(&format!(r".\{}.zip", cli.sid.unwrap())),
    )
    .await?;

    println!("下载完成");
    Ok(())
}

fn read_config() -> Result<(UserSession, PathBuf)> {
    let mut dir = env::current_dir()?;
    dir.push("config.json");
    let file = dir.as_path();

    if !file.is_file() {
        let file = fs::File::create(file);
        if file.is_err() {
           anyhow!("创建配置文件失败,请检查程序目录下是否有名为'config.json'的文件夹");
        }
        let mut file = file.unwrap();
        let config = Config{
            username:String::new(),
            password:String::new(),
            session:String::new(),
        };
        let config_str =
            serde_json::to_string(&config)?;
        file.write_all(config_str.as_bytes())?;
        anyhow!("未检测到配置文件,已经在程序所在目录下生产了模板配置文件,请完善账号信息");
    }

    let config = fs::read(file).with_context(|| "读取用户配置失败")?;
    let config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败，请删除掉配置文件重试！")?;
    let mut user = UserSession::new(&config.username,&config.password);
    user.read_session(&config.session);
    return Ok((user, dir))
}

fn save_user_cookie(user: &mut UserSession, user_config_path: &Path) -> Result<()> {
    let config = fs::read(user_config_path).with_context(|| "读取用户配置失败")?;
    let mut  config: Config = serde_json::from_slice(&config)
        .with_context(|| "解析用户配置失败，请删除掉配置文件重试！")?;
    config.session = user.save_session();
    let config_str =
        serde_json::to_string(&config)?;
    fs::write(user_config_path, config_str.as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
