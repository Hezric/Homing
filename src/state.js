export const localeFor = (language) => (language?.toLowerCase().startsWith("zh") ? "zh" : "en");
export function mockState(status = "connected") {
  return {
    current:
      status === "disconnected" || status === "reconnecting" || status === "unreadable"
        ? null
        : "Home Wi-Fi",
    profile: "Home Wi-Fi",
    connection: status === "paused" ? "connected" : status,
    pauseRemaining: status === "paused" ? 3600 : 0,
    pauseMinutes: 60,
    lastReconnect: 1793310720,
    languageMode: "system",
    error: null,
  };
}
export function mockUpdate(state, action) {
  if (action.type === "setProfile") state.profile = action.profile;
  if (action.type === "setPauseMinutes") state.pauseMinutes = action.minutes;
  if (action.type === "setLanguage") state.languageMode = action.language;
  if (action.type === "togglePause")
    state.pauseRemaining = state.pauseRemaining ? 0 : state.pauseMinutes * 60;
  return state;
}
export const text = {
  en: {
    app: "Homing Wi-Fi reconnect",
    none: "No Wi-Fi connection",
    unreadable: "Can't read Wi-Fi state",
    connected: "Connected",
    reconnecting: "Reconnecting…",
    disconnected: "Can't reach",
    paused: "Paused",
    remaining: "resumes in",
    minutes: "min",
    resume: "Resume now",
    settings: "Settings",
    language: "Language",
    system: "Follow system",
    pauseFor: "Pause for",
    pauseError: "Enter a whole number from 1 to 720.",
    target: "Reconnect to",
    last: "Last reconnect",
    never: "Not yet",
    watching: "Watching for disconnects",
    retrying: "Trying your saved network",
    waiting: "Waiting to retry",
    setup: "Choose a saved Wi-Fi profile in Settings.",
    profile: "Saved Wi-Fi profile",
    profileHint: "Use the saved profile name from Windows Wi-Fi settings.",
    profileError: "Enter a profile name of 1–255 characters.",
    save: "Save",
    failed: "Could not save changes. Try again.",
    attention: "Reconnect needs attention",
    github: "Open Homing on GitHub",
    reconnect: "Auto-reconnect",
    off: "Not configured",
  },
  zh: {
    app: "Homing Wi-Fi 自动重连",
    none: "未连接 Wi-Fi",
    unreadable: "无法读取 Wi-Fi 状态",
    connected: "已连接",
    reconnecting: "正在重连…",
    disconnected: "无法连接",
    paused: "已暂停",
    remaining: "恢复还需",
    minutes: "分钟",
    resume: "立即恢复",
    settings: "设置",
    language: "语言",
    system: "跟随系统",
    pauseFor: "暂停时长",
    pauseError: "请输入 1 到 720 之间的整数。",
    target: "重连网络",
    last: "上次重连",
    never: "暂无",
    watching: "正在监测连接",
    retrying: "正在连接已保存的网络",
    waiting: "等待重试",
    setup: "请在设置中填写已保存的 Wi-Fi 配置名称。",
    profile: "已保存的 Wi-Fi 配置",
    profileHint: "请填写 Windows Wi-Fi 设置中已保存的配置名称。",
    profileError: "请输入 1 到 255 个字符的配置名称。",
    save: "保存",
    failed: "无法保存更改，请重试。",
    attention: "重连需要处理",
    github: "在 GitHub 上打开 Homing",
    reconnect: "自动重连",
    off: "未配置",
  },
};
export function parsePauseMinutes(value) {
  const text = value.trim(),
    minutes = Number(text);
  return /^\d+$/.test(text) && minutes >= 1 && minutes <= 720 ? minutes : null;
}
export function validProfile(value) {
  return value.trim().length > 0 && value.length <= 255 && !/[\0\r\n]/.test(value);
}
export function pauseLabel(minutes, language) {
  return language === "zh"
    ? minutes % 60 === 0
      ? `暂停 ${minutes / 60} 小时`
      : `暂停 ${minutes} 分钟`
    : minutes % 60 === 0
      ? `Pause ${minutes / 60} hr`
      : `Pause ${minutes} min`;
}
export function reconnectLabel(state, language) {
  const t = text[language];
  if (state.connection === "unreadable") return t.unreadable;
  if (!state.profile)
    return language === "zh" ? "选择要保持连接的网络" : "Choose a network to keep";
  if (state.connection === "connected") return t.watching;
  if (state.connection === "reconnecting") return t.reconnecting;
  return language === "zh"
    ? `无法连接 ${state.profile} · 自动重试中`
    : `Can't reach ${state.profile} · retrying automatically`;
}
export const connectionLabel = (state, language) =>
  state.connection === "unreadable"
    ? text[language].unreadable
    : state.current
      ? text[language].connected
      : reconnectLabel(state, language);
