use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Result;

/// 解压zip文件,.osz实际上是zip封装的压缩包
pub fn unzip(zip_path: &Path, path: PathBuf) -> Result<()> {
    let zip_file = std::fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(zip_file)?;

    if !path.exists() {
        fs::create_dir_all(&path)?;
    }

    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let outpath = match f.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if false {
            // 压缩包说明
            let comment = f.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (*f.name()).ends_with('/') {
            // 文件夹
            println!("释放文件夹{}", outpath.display());
            fs::create_dir_all(outpath.as_path())?;
        } else {
            // 文件
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            println!("释放文件{}", outpath.display());
            let mut outfile = fs::File::create(path.join(outpath))?;
            io::copy(&mut f, &mut outfile)?;
        }
    }
    Ok(())
}
