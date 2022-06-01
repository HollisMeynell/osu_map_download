use osu_map_download::util::{download, UserSession};
use std::path::Path;

#[tokio::main]
async fn main() {
    let mut puser = UserSession::new("your osu name", "your password");

    let path = Path::new(r"D:\rt2.zip");
    download(1767138, &mut puser, path).await.unwrap();
    let path = Path::new(r"D:\rt3.zip");
    download(1574263, &mut puser, path).await.unwrap();
    let path = Path::new(r"D:\rt1.zip");
    download(1518105, &mut puser, path).await.unwrap();
}
