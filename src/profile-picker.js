import { text, validNetwork } from "./state.js";

export function createProfilePicker(container, { load, save }) {
  container.innerHTML = `<div class="profile-heading"><h2><label id="profile-label" for="profile"></label></h2><button id="use-current-profile" class="button" type="button" aria-describedby="current-profile-hint"></button></div>
    <div class="profile-row"><input id="profile" class="profile" type="text" maxlength="32" autocomplete="off" spellcheck="false" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="profile-list" aria-describedby="profile-hint profile-error"><button class="button" id="save-profile" type="button"></button></div>
    <p id="profile-hint" class="profile-hint"></p><p id="current-profile-hint" class="profile-hint"></p><button id="retry-profiles" class="button" type="button" hidden></button>
    <p id="profile-error" class="profile-error" role="alert" hidden></p>
    <div id="profile-popup" class="profile-popup" popover="manual"><div id="profile-list" role="listbox" aria-labelledby="profile-label"></div><p id="profile-list-status" role="status"></p></div>`;
  const input = container.querySelector("#profile");
  const popup = container.querySelector("#profile-popup");
  const list = container.querySelector("#profile-list");
  const status = container.querySelector("#profile-list-status");
  const error = container.querySelector("#profile-error");
  const currentButton = container.querySelector("#use-current-profile");
  const saveButton = container.querySelector("#save-profile");
  const retry = container.querySelector("#retry-profiles");
  let language = "en",
    saved = "",
    dirty = false,
    opened = false,
    active = -1,
    revision = 0;
  let catalog = null,
    loadError = false,
    loading = null,
    saving = false,
    errorKey = "";
  let matches = [];
  function position() {
    if (!opened) return;
    const row = input.getBoundingClientRect();
    const footer = document.querySelector("footer").getBoundingClientRect();
    const available = Math.max(
      0,
      Math.min(220, footer.top - row.bottom - 4, innerHeight - row.bottom - 4),
    );
    popup.style.left = `${row.left}px`;
    popup.style.top = `${row.bottom}px`;
    popup.style.width = `${row.width}px`;
    popup.style.maxHeight = `${available}px`;
    list.style.maxHeight = `${Math.max(0, available - 10)}px`;
  }
  function close() {
    opened = false;
    active = -1;
    if (popup.matches(":popover-open")) popup.hidePopover();
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
  }
  function draw() {
    const t = text[language];
    container.querySelector("#profile-label").textContent = t.profile;
    container.querySelector("#profile-hint").textContent = t.profileHint;
    currentButton.textContent = t.useCurrent;
    const current = catalog?.current && catalog.names.includes(catalog.current);
    currentButton.disabled = saving || !!loading || !current;
    const currentHint = container.querySelector("#current-profile-hint");
    currentButton.title = t.currentProfileReady;
    currentHint.hidden = !!current && !loadError;
    currentHint.textContent = loading
      ? t.profilesLoading
      : loadError
        ? t.networksUnavailable
        : current
          ? t.currentProfileReady
          : t.currentProfileHint;
    saveButton.textContent = t.save;
    saveButton.disabled = saving;
    error.textContent = errorKey ? t[errorKey] || t.failed : "";
    error.hidden = !errorKey;
    input.setAttribute("aria-invalid", String(!!errorKey));
    input.setAttribute("aria-busy", String(!!loading));
    const query = input.value.toLocaleLowerCase();
    matches = (catalog?.names || []).filter((name) => name.toLocaleLowerCase().includes(query));
    if (active >= matches.length) active = -1;
    list.replaceChildren(
      ...matches.map((name, index) => {
        const option = document.createElement("div");
        option.id = `profile-option-${index}`;
        option.className = "profile-option";
        option.setAttribute("role", "option");
        option.setAttribute("aria-selected", String(index === active));
        option.title = name;
        option.textContent = name;
        option.addEventListener("pointerdown", (event) => event.preventDefault());
        option.addEventListener("click", () => select(name));
        return option;
      }),
    );
    status.textContent = loading
      ? t.profilesLoading
      : loadError
        ? t.networksUnavailable
        : !matches.length
          ? t.profilesEmpty
          : "";
    status.hidden = !status.textContent;
    retry.hidden = !loadError;
    retry.disabled = !!loading;
    retry.textContent = t.retryProfiles;
    if (opened) {
      if (!popup.matches(":popover-open")) popup.showPopover();
      input.setAttribute("aria-expanded", "true");
      position();
      if (active >= 0) {
        const option = list.children[active];
        input.setAttribute("aria-activedescendant", option.id);
        option.scrollIntoView({ block: "nearest" });
      } else input.removeAttribute("aria-activedescendant");
    }
  }
  async function refresh() {
    if (loading) return loading;
    loading = Promise.resolve().then(load);
    loadError = false;
    draw();
    try {
      catalog = await loading;
    } catch {
      catalog = null;
      loadError = true;
    } finally {
      loading = null;
      draw();
    }
  }
  function select(name) {
    revision++;
    input.value = name;
    dirty = true;
    errorKey = "";
    close();
    draw();
  }
  async function submit(useCurrent = false) {
    close();
    if (!useCurrent && !validNetwork(input.value)) {
      errorKey = "profileError";
      draw();
      return;
    }
    saving = true;
    errorKey = "";
    draw();
    const submittedRevision = revision;
    const result = await save(useCurrent ? null : input.value);
    saving = false;
    if (result.ok) {
      saved = result.profile;
      if (revision === submittedRevision) {
        dirty = false;
        input.value = saved;
      }
    } else {
      const code = String(result.error?.message || result.error);
      errorKey = [
        "networkInvalid",
        "networkNotSaved",
        "networkAmbiguous",
        "networksUnavailable",
        "currentNetworkUnavailable",
      ].includes(code)
        ? code === "networkInvalid"
          ? "profileError"
          : code
        : "failed";
    }
    draw();
  }
  input.addEventListener("focus", () => {
    opened = true;
    draw();
    refresh();
  });
  input.addEventListener("input", () => {
    revision++;
    dirty = true;
    errorKey = "";
    active = -1;
    opened = true;
    draw();
  });
  input.addEventListener("keydown", (event) => {
    if (event.isComposing) {
      event.stopPropagation();
      return;
    }
    if (["ArrowDown", "ArrowUp"].includes(event.key)) {
      event.preventDefault();
      opened = true;
      active = matches.length
        ? event.key === "ArrowDown"
          ? (active + 1) % matches.length
          : active <= 0
            ? matches.length - 1
            : active - 1
        : -1;
      draw();
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (opened && active >= 0) select(matches[active]);
      else submit();
    } else if (event.key === "Escape" && opened) {
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (event.key === "Tab") close();
  });
  input.addEventListener("blur", (event) => {
    if (!popup.contains(event.relatedTarget)) close();
  });
  document.addEventListener("pointerdown", (event) => {
    if (!container.contains(event.target)) close();
  });
  saveButton.addEventListener("click", () => submit());
  currentButton.addEventListener("click", () => submit(true));
  retry.addEventListener("click", refresh);
  window.addEventListener("resize", position);
  document.addEventListener("scroll", position, true);
  new ResizeObserver(position).observe(document.querySelector(".flyout"));
  return {
    sync(profile, locale) {
      saved = profile;
      language = locale;
      if (!dirty) input.value = saved;
      draw();
    },
    open: refresh,
    close,
  };
}
