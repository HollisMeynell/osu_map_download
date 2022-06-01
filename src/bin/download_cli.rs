use anyhow::{Context, Result};
use clap::Parser;
use osu_map_download::util::{do_download, UserSession};
use std::fs;
use std::io::Write;

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

async fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.username.is_none() || cli.password.is_none() {
        println!("正在从配置文件中读取用户信息......")
    }

    let mut user = UserSession::new(&cli.username.unwrap(), &cli.password.unwrap());

    println!("正在下载...");

    let p = do_download(cli.sid, &mut user).await?;
    let mut file = fs::File::create(format!(r".\{}.zip", cli.sid))?;
    file.write(&p).with_context(|| "文件写入失败")?;

    println!("下载完成");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
