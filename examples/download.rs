use std::fs;
use std::io::Write;
use osu_map_download::util::{do_download, UserSession};

#[tokio::main]
async fn main() {
    let mut puser = UserSession::new("your osu name", "your password");


    let p = do_download(1767138, &mut puser).await.unwrap();
    let mut file = fs::File::create("D:\\rt2.zip").unwrap();
    file.write(p.as_ref()).unwrap();
    let p = do_download(1574263, &mut puser).await.unwrap();
    let mut file = fs::File::create("D:\\rt3.zip").unwrap();
    file.write(p.as_ref()).unwrap();
    let p = do_download(1518105, &mut puser).await.unwrap();
    let mut file = fs::File::create("D:\\rt1.zip").unwrap();
    file.write(p.as_ref()).unwrap();
}