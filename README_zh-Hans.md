# Homing

让 Wi-Fi 静静归来。

[English](README.md)

Homing 守候你已保存的 Windows Wi-Fi。已观测的网络名称中找不到指定名称时，它便请
Windows 重新连接。平时由 WLAN 通知唤醒，重试和暂停到期由定时器处理。

小巧的托盘面板展示当前网络、重连状态与上次重连请求的时间。齿轮后的设置可填写
配置名称、选择 1–720 分钟的暂停时长，以及英语或简体中文。主题与强调色跟随
Windows。带循环标记的托盘图标便于区分系统 Wi-Fi，并跟随任务栏尺寸。
任务栏图标的面板居中显示在图标上方，隐藏图标则从屏幕侧边滑入通知区角落。阴影与圆角由 Windows 绘制；点击外部、按 Esc 或再次点击图标都会关闭面板。关闭系统动画时立即切换。

## 安装

从[发布页](https://github.com/Hezric/Homing/releases/latest)下载 Windows x64 便携包，
对照 `SHA256SUMS` 校验后解压，并保留 WebView2。保存目标后即启用自动重连。
登录启动交给已有启动管理器。

## 构建

需要 Windows 11、Rust MSVC 工具链、Visual Studio C++ Build Tools、Node.js LTS
和 Microsoft Edge WebView2，以及 [Vite+](https://viteplus.dev/guide/)。

```powershell
git clone https://github.com/Hezric/Homing.git
cd Homing
vp install --frozen-lockfile
vp exec tauri build --ci --no-bundle -- --locked
```

便携程序位于 `src-tauri/target/release/homing.exe`。登录启动交给 Lineup 等启动管理器。

### 交给智能体

将下面这段话交给能够构建 Windows 软件的智能体：

```text
从 https://github.com/Hezric/Homing/releases/latest 安装 Homing。
对照 SHA256SUMS 校验 Windows x64 便携包，解压并确认 homing.exe 存在，先不启动。
保留已有设置和登录启动登记。说明保存目标或使用已有目标会启用 Wi-Fi 重连，激活前取得授权。
通过我已有的 Skill 管理方式安装 skills/homing。若没有，请先询问，建议使用
Vercel skills CLI：npx skills add Hezric/Homing -g（https://github.com/vercel-labs/skills，
要求 Node.js 22.20 及以上）。也要确认安装后 Skill 的位置和完整内容。
```

## 使用

单击托盘图标开关面板，右键可打开、暂停或恢复、退出。进入齿轮设置，填写
**已保存的 Wi-Fi 配置名称**并保存，自动重连即刻生效。Windows 必须已经保存
该配置及其凭据。Homing 不会创建配置、扫描网络或索要密码。

名称匹配保留原工具的规则：在已观测的 Windows 网络名称中进行
区分大小写的子串查找。例如 `Home` 也会匹配 `MyHome5G`。找不到时，以不经过命令
解释器的方式运行 `netsh wlan connect name=<profile>`。请求被接受后，等待一秒再
检查；命令失败后，等待 61 秒。请求被接受并不代表连接已完成。**上次重连**表示本次
运行中最近一次被接受的请求，重启后清空。

暂停预设为 15、30、60、120 分钟，默认为 60 分钟；自定义时长在回车或离开输入框时
保存。修改时长不会改变正在进行的暂停。重启后恢复重连，暂停本身不会断开 Wi-Fi。

设置保存在 `%APPDATA%\Homing\Homing.ini`，配置名称以 UTF-8 十六进制编码存储，
保留中文、空格与 INI 标点。更改以原子方式保存。配置缺失或损坏时不重连；损坏的
原文件会保留到你主动保存设置。错误会显示在面板中。WLAN 通知不可用时，会暂时定时检查并
重试注册；通知恢复后停止轮询。Wi-Fi 适配器状态和原始 Windows 网络描述不需要位置服务权限；
显示与匹配使用该原始名称，不受 Windows 添加的连接配置名称后缀影响。无法读取状态时，托盘和面板显示**无法读取 Wi-Fi 状态**。
此时只重试观测，不会连接；只有确认目标网络未连接时才会请求重连。

“无法连接”表示目标网络未连接，程序会自动重试；没有目标时提示选择网络，“正在重连”表示已发送请求、等待结果。实心表示工作中，空心表示暂不可用，两条竖线表示暂停；所有托盘状态保持完整对比度。

## 预览与参与

```powershell
vp run dev
```

打开 `http://127.0.0.1:1420/?lang=zh&theme=light&state=connected`。
浏览器使用模拟数据，不会更改 Windows Wi-Fi。`lang` 可选 `en` 或 `zh`，`theme`
可选 `light` 或 `dark`，`state` 可选 `connected`、`reconnecting`、`disconnected`
、`unreadable` 或 `paused`；`view=settings` 显示设置页。

[参与贡献](CONTRIBUTING.md) · [行为与检查](docs/features/wifi-reconnect.md)
· [MIT 许可](LICENSE) · [第三方声明](THIRD-PARTY-NOTICES.md)
