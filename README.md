# Osu Beatmap Downloader

![Image](./images/screenshot.png)

一个用 Rust 写的轻量 OSU 谱面下载器。

## 版本

我们提供了两个版本的 cli。默认不保存密码，一切都用 cookie 进行操作。
你也可以选择带保存密码的版本，密码将会保存在系统的密码管理器里。
当 cookie 过期时程序就会自动查询密码自动重新登录，而无需你再次手动输入密码。
