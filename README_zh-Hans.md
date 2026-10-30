# Homing

让 Wi-Fi 静静归来。

[English](README.md)

Homing 守候你已保存的 Windows Wi-Fi。已观测的网络名称中找不到指定名称时，它便请
Windows 重新连接。平时由 WLAN 通知唤醒，重试和暂停到期由定时器处理。

小巧的托盘面板展示当前网络、重连状态与上次重连请求的时间。齿轮后的设置可填写
配置名称、选择 1–720 分钟的暂停时长，以及英语或简体中文。主题与强调色跟随
Windows。带角标的托盘图标便于区分系统 Wi-Fi，并跟随任务栏尺寸。
面板居中显示在托盘图标上方，使用短动画；关闭系统动画效果时立即切换，关闭后释放网页视图。

## 构建

需要 Windows 11、Rust MSVC 工具链、Visual Studio C++ Build Tools、Node.js LTS
和 Microsoft Edge WebView2。目前尚无发布版本。

```powershell
git clone https://github.com/Hezric/Homing.git
cd Homing
npm ci
npm run tauri -- build --ci --no-bundle -- --locked
```

便携程序位于 `src-tauri/target/release/homing.exe`。登录启动交给 Lineup 等启动管理器。

### 交给智能体

将下面这段话交给能够构建 Windows 软件的智能体：

```text
请从 https://github.com/Hezric/Homing 获取 Homing，在本地构建。
先读 AGENTS.md，检查 Windows 11、MSVC Rust/C++ 工具链、Node.js LTS
和 WebView2。运行 npm ci，然后运行
npm run tauri -- build --ci --no-bundle -- --locked。
确认 src-tauri/target/release/homing.exe 已生成，但不要启动。
说明保存 Wi-Fi 配置名称后会启用重连，也可能切换网络。
使用已有配置启动前，或在承载远程会话的机器上更改 Wi-Fi 前，先征得同意。
保持登录启动设置不变。
```

## 使用

单击托盘图标打开面板，右键可打开、暂停或恢复、退出。进入齿轮设置，填写
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

## 预览与参与

```powershell
npm run dev
```

打开 `http://127.0.0.1:1420/?lang=zh&theme=light&state=connected`。
浏览器使用模拟数据，不会更改 Windows Wi-Fi。`lang` 可选 `en` 或 `zh`，`theme`
可选 `light` 或 `dark`，`state` 可选 `connected`、`reconnecting`、`disconnected`
、`unreadable` 或 `paused`；`view=settings` 显示设置页。

[参与贡献](CONTRIBUTING.md) · [行为与检查](docs/features/wifi-reconnect.md)
· [MIT 许可](LICENSE) · [第三方声明](THIRD-PARTY-NOTICES.md)
