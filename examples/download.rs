use osu_map_download::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() {
    let mut puser = UserSession::new("your osu name", "your password")
        .await
        .unwrap();

    let path = Path::new(r"D:\rt2.zip");
    let pending = vec![
        "1767138".to_string(),
        "1574263".to_string(),
        "1518105".to_string(),
    ];

    download(&pending, &mut puser, path, true).await.unwrap();
}
