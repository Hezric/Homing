export const localeFor = (language) => (language?.toLowerCase().startsWith("zh") ? "zh" : "en");
export function mockState(status = "connected") {
  return {
    current:
      status === "disconnected" || status === "reconnecting" || status === "unreadable"
        ? null
        : "Home Wi-Fi",
    profile: "Hidden Home alias",
    targetName: "Home Wi-Fi",
    configured: true,
    connection: status === "paused" ? "connected" : status,
    pauseRemaining: status === "paused" ? 3600 : 0,
    pauseMinutes: 60,
    lastReconnect: 1793310720,
    languageMode: "system",
    error: null,
  };
}
export function mockSavedNetworks(mode = "") {
  if (mode === "unavailable") throw new Error("networksUnavailable");
  const names =
    mode === "empty"
      ? []
      : [
          "Home Wi-Fi",
          "Office Wi-Fi",
          "家里的网络",
          "<Office & Guest>",
          "Long" + "W".repeat(28),
          ...Array.from(
            { length: 30 },
            (_, index) => `Studio ${String(index + 1).padStart(2, "0")}`,
          ),
        ];
  if (mode === "stale") names.push("Removed network");
  if (mode === "ambiguous") names.push("Shared Wi-Fi");
  return {
    names,
    current: ["no-current", "current-mismatch", "empty", "ambiguous"].includes(mode)
      ? null
      : "Home Wi-Fi",
  };
}
export function mockUpdate(state, action, networkMode = "") {
  if (["setNetwork", "useCurrentNetwork"].includes(action.type)) {
    if (action.type === "setNetwork" && !validNetwork(action.network))
      throw new Error("networkInvalid");
    const saved = mockSavedNetworks(networkMode === "stale" ? "" : networkMode);
    const network = action.type === "useCurrentNetwork" ? saved.current : action.network;
    if (!network) throw new Error("currentNetworkUnavailable");
    if (!saved.names.includes(network)) throw new Error("networkNotSaved");
    if (networkMode === "ambiguous" && network === "Shared Wi-Fi")
      throw new Error("networkAmbiguous");
    state.profile = `Hidden alias for ${network}`;
    state.targetName = network;
    state.configured = true;
  }
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
    setup: "Choose a saved Wi-Fi network in Settings.",
    profile: "Saved Wi-Fi network",
    profileHint: "Search by the Wi-Fi name shown in Windows. Select, then Save.",
    useCurrent: "Use current",
    currentProfileHint:
      "The current network cannot be matched to one saved entry. Choose a network below.",
    currentProfileReady: "Save the network currently connected in Windows.",
    profilesLoading: "Loading saved networks…",
    networksUnavailable: "Could not read saved networks. Retry the list before saving.",
    profilesEmpty: "No matching saved networks. Check the name in Windows Wi-Fi settings.",
    networkNotSaved:
      "This network is not saved in Windows. Check its Wi-Fi name or connect to it in Windows first.",
    currentNetworkUnavailable:
      "The current saved network is no longer available. Choose a network below.",
    retryProfiles: "Retry",
    networkAmbiguous:
      "Windows has more than one saved entry with this name. Connect to the intended network in Windows, then retry.",
    targetUnavailable: "Saved network name unavailable",
    profileError: "Enter the Wi-Fi network name shown in Windows (up to 32 bytes).",
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
    setup: "请在设置中选择已保存的 Wi-Fi 网络。",
    profile: "已保存的 Wi-Fi 网络",
    profileHint: "输入系统 Wi-Fi 中显示的网络名，选好后点击保存。",
    useCurrent: "使用当前网络",
    currentProfileHint: "无法唯一对应当前连接的已保存网络，请从下方选择。",
    currentProfileReady: "保存 Windows 当前已连接的网络。",
    profilesLoading: "正在读取已保存的网络…",
    networksUnavailable: "无法读取已保存的网络，请重试列表后再保存。",
    profilesEmpty: "没有匹配的已保存网络，请在 Windows Wi-Fi 设置中核对名称。",
    networkNotSaved: "Windows 中没有保存此网络，请核对系统 Wi-Fi 中的名称。",
    currentNetworkUnavailable: "当前连接的已保存网络已不可用，请从下方选择。",
    retryProfiles: "重试",
    networkAmbiguous: "Windows 中有多个同名的已保存网络，请在系统 Wi-Fi 中连接需要的网络后重试。",
    targetUnavailable: "无法读取已保存网络名",
    profileError: "请输入系统 Wi-Fi 中的网络名，最多 32 字节。",
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
export function validNetwork(value) {
  return (
    value.trim().length > 0 &&
    new TextEncoder().encode(value).length <= 32 &&
    !/[\0\r\n]/.test(value)
  );
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
  if (!state.configured)
    return language === "zh" ? "选择要保持连接的网络" : "Choose a network to keep";
  if (state.connection === "connected") return t.watching;
  if (state.connection === "reconnecting") return t.reconnecting;
  return language === "zh"
    ? `无法连接 ${state.targetName || t.targetUnavailable} · 自动重试中`
    : `Can't reach ${state.targetName || t.targetUnavailable} · retrying automatically`;
}
export const connectionLabel = (state, language) =>
  state.connection === "unreadable"
    ? text[language].unreadable
    : state.current
      ? text[language].connected
      : reconnectLabel(state, language);
