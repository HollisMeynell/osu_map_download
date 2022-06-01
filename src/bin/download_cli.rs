use anyhow::{Context, Result};
use clap::Parser;
use directories::BaseDirs;
use osu_map_download::util::{download, UserSession};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Parser)]
#[clap(name = "osu beatmap downloader")]
#[clap(author = "-Sprint Night-, CookieBacon")]
#[clap(version = "0.1")]
#[clap(about = "A cli to help your download osu beatmap")]
struct Cli {
    sid: u64,
    #[clap(long)]
    username: Option<String>,
    #[clap(long)]
    password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    username: String,
    password: String,
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

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
        cli.sid,
        &mut user,
        Path::new(&format!(r".\{}.zip", cli.sid)),
    )
    .await?;

    println!("下载完成");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
