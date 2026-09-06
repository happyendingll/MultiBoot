import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

interface CommandItem {
  id: string;
  title: string;
  command: string;
  enabled: boolean;
  order: number;
  icon?: string;
}

interface AppConfig {
  version: number;
  settings: { autostart: boolean };
  items: CommandItem[];
}

interface ConfigSnapshot {
  config: AppConfig;
  path: string;
  warnings: string[];
}

interface CommandResult {
  success: boolean;
  exitCode: number | null;
  stdout: string;
  stderr: string;
}

let currentConfig: AppConfig = { version: 1, settings: { autostart: true }, items: [] };
let editorIcon: string | null = null;

const requireElement = <T extends Element>(selector: string): T => {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`缺少页面元素：${selector}`);
  return element;
};

const button = (text: string, action: () => void, disabled = false): HTMLButtonElement => {
  const element = document.createElement("button");
  element.type = "button";
  element.textContent = text;
  element.disabled = disabled;
  element.addEventListener("click", action);
  return element;
};

const applySnapshot = (snapshot: ConfigSnapshot): void => {
  currentConfig = snapshot.config;
  requireElement<HTMLElement>("#config-path").textContent = snapshot.path;
  const warnings = requireElement<HTMLElement>("#startup-warnings");
  warnings.replaceChildren();
  warnings.hidden = snapshot.warnings.length === 0;
  for (const warning of snapshot.warnings) {
    const paragraph = document.createElement("p");
    paragraph.textContent = warning;
    warnings.append(paragraph);
  }
  renderEntries();
};

const renderEntries = (): void => {
  const container = requireElement<HTMLDivElement>("#entries");
  const summary = requireElement<HTMLParagraphElement>("#entry-summary");
  const items = [...currentConfig.items].sort((a, b) => a.order - b.order);
  container.classList.remove("error-message");
  container.replaceChildren();

  if (items.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty";
    empty.innerHTML = "<strong>暂无命令条目</strong><span>点击“新增条目”创建第一个托盘命令。</span>";
    container.append(empty);
    summary.textContent = "0 个条目；托盘菜单仅显示“设置”和“退出”。";
    return;
  }

  const enabledCount = items.filter((item) => item.enabled).length;
  summary.textContent = `${items.length} 个条目，${enabledCount} 个已启用`;
  items.forEach((item, index) => {
    const article = document.createElement("article");
    article.className = item.enabled ? "entry" : "entry disabled";

    const info = document.createElement("div");
    info.className = "entry-info";
    const heading = document.createElement("div");
    heading.className = "entry-heading";
    const title = document.createElement("h3");
    title.textContent = item.title;
    if (item.icon) {
      const icon = document.createElement("img");
      icon.className = "entry-icon";
      icon.src = item.icon;
      icon.alt = "";
      heading.append(icon);
    }
    const state = document.createElement("span");
    state.className = `mini-status ${item.enabled ? "on" : "off"}`;
    state.textContent = item.enabled ? "已启用" : "已停用";
    heading.append(title, state);
    const command = document.createElement("code");
    command.textContent = item.command;
    info.append(heading, command);

    const actions = document.createElement("div");
    actions.className = "entry-actions";
    actions.append(
      button("上移", () => void moveItem(item.id, "up"), index === 0),
      button("下移", () => void moveItem(item.id, "down"), index === items.length - 1),
      button(item.enabled ? "停用" : "启用", () => void toggleItem(item)),
      button("编辑", () => openEditor(item)),
      button("删除", () => void deleteItem(item), false),
    );
    actions.lastElementChild?.classList.add("danger");
    article.append(info, actions);
    container.append(article);
  });
};

const showMutationError = (error: unknown): void => {
  const message = String(error);
  requireElement<HTMLElement>("#form-error").textContent = message;
  if (requireElement<HTMLElement>("#editor-panel").hidden) {
    const warnings = requireElement<HTMLElement>("#startup-warnings");
    const paragraph = document.createElement("p");
    paragraph.textContent = message;
    warnings.replaceChildren(paragraph);
    warnings.hidden = false;
  }
};

const loadConfig = async (): Promise<void> => {
  try {
    applySnapshot(await invoke<ConfigSnapshot>("get_config"));
  } catch (error) {
    const entries = requireElement<HTMLDivElement>("#entries");
    entries.textContent = `读取配置失败：${String(error)}`;
    entries.classList.add("error-message");
  }
};

const loadAutostart = async (): Promise<void> => {
  const toggle = requireElement<HTMLInputElement>("#autostart-toggle");
  const hint = requireElement<HTMLElement>("#autostart-hint");
  try {
    const enabled = await invoke<boolean>("get_autostart_enabled");
    toggle.checked = enabled;
    toggle.disabled = false;
    hint.textContent = enabled ? "已在当前系统注册自启动。" : "当前系统未注册自启动。";
  } catch (error) {
    toggle.disabled = true;
    hint.textContent = String(error);
  }
};

const changeAutostart = async (enabled: boolean): Promise<void> => {
  const toggle = requireElement<HTMLInputElement>("#autostart-toggle");
  const hint = requireElement<HTMLElement>("#autostart-hint");
  toggle.disabled = true;
  hint.textContent = "正在更新系统自启动状态…";
  try {
    const actual = await invoke<boolean>("set_autostart_enabled", { enabled });
    toggle.checked = actual;
    hint.textContent = actual ? "已在当前系统注册自启动。" : "当前系统未注册自启动。";
  } catch (error) {
    hint.textContent = String(error);
    await loadAutostart();
  } finally {
    toggle.disabled = false;
  }
};

const exportItems = async (): Promise<void> => {
  const status = requireElement<HTMLElement>("#transfer-status");
  const path = await save({
    title: "导出 MultiBoot 配置",
    defaultPath: "multiboot-config.json",
    filters: [{ name: "JSON 配置", extensions: ["json"] }],
  });
  if (!path) return;
  try {
    await invoke("export_config", { path });
    status.textContent = `已导出 ${currentConfig.items.length} 个条目。`;
  } catch (error) {
    status.textContent = String(error);
  }
};

const importItems = async (): Promise<void> => {
  const status = requireElement<HTMLElement>("#transfer-status");
  const mode = requireElement<HTMLSelectElement>("#import-mode").value;
  if (mode === "replace" && !window.confirm("覆盖导入会删除当前所有业务条目，是否继续？")) return;
  const path = await open({
    title: "导入 MultiBoot 配置",
    multiple: false,
    directory: false,
    filters: [{ name: "JSON 配置", extensions: ["json"] }],
  });
  if (!path) return;
  try {
    applySnapshot(await invoke<ConfigSnapshot>("import_config", { path, mode }));
    status.textContent = `导入成功，当前共 ${currentConfig.items.length} 个条目；托盘已刷新。`;
  } catch (error) {
    status.textContent = String(error);
  }
};

const moveItem = async (id: string, direction: "up" | "down"): Promise<void> => {
  try {
    applySnapshot(await invoke<ConfigSnapshot>("move_entry", { id, direction }));
  } catch (error) {
    showMutationError(error);
  }
};

const toggleItem = async (item: CommandItem): Promise<void> => {
  try {
    applySnapshot(
      await invoke<ConfigSnapshot>("set_entry_enabled", {
        id: item.id,
        enabled: !item.enabled,
      }),
    );
  } catch (error) {
    showMutationError(error);
  }
};

const deleteItem = async (item: CommandItem): Promise<void> => {
  if (!window.confirm(`确定删除“${item.title}”吗？`)) return;
  try {
    applySnapshot(await invoke<ConfigSnapshot>("delete_entry", { id: item.id }));
  } catch (error) {
    showMutationError(error);
  }
};

const openEditor = (item?: CommandItem): void => {
  requireElement<HTMLElement>("#editor-panel").hidden = false;
  requireElement<HTMLElement>("#editor-title").textContent = item ? "编辑条目" : "新增条目";
  requireElement<HTMLInputElement>("#entry-id").value = item?.id ?? "";
  requireElement<HTMLInputElement>("#entry-title").value = item?.title ?? "";
  requireElement<HTMLTextAreaElement>("#entry-command").value = item?.command ?? "";
  requireElement<HTMLInputElement>("#entry-enabled").checked = item?.enabled ?? true;
  editorIcon = item?.icon ?? null;
  renderIconPreview();
  requireElement<HTMLElement>("#form-error").textContent = "";
  requireElement<HTMLInputElement>("#entry-title").focus();
};

const closeEditor = (): void => {
  requireElement<HTMLElement>("#editor-panel").hidden = true;
  requireElement<HTMLFormElement>("#entry-form").reset();
  requireElement<HTMLElement>("#form-error").textContent = "";
};

const saveEditor = async (): Promise<void> => {
  const id = requireElement<HTMLInputElement>("#entry-id").value;
  const input = {
    id: id || null,
    title: requireElement<HTMLInputElement>("#entry-title").value,
    command: requireElement<HTMLTextAreaElement>("#entry-command").value,
    enabled: requireElement<HTMLInputElement>("#entry-enabled").checked,
    icon: editorIcon,
  };
  try {
    applySnapshot(await invoke<ConfigSnapshot>("save_entry", { input }));
    closeEditor();
  } catch (error) {
    showMutationError(error);
  }
};

const renderIconPreview = (): void => {
  const preview = requireElement<HTMLDivElement>("#icon-preview");
  preview.replaceChildren();
  if (!editorIcon) {
    preview.textContent = "无";
    return;
  }
  const image = document.createElement("img");
  image.src = editorIcon;
  image.alt = "当前图标预览";
  preview.append(image);
};

const loadIcon = (file: File): Promise<string> =>
  new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      typeof reader.result === "string" ? resolve(reader.result) : reject(new Error("读取图片失败"));
    });
    reader.addEventListener("error", () => reject(reader.error ?? new Error("读取图片失败")));
    reader.readAsDataURL(file);
  });

const renderResult = (result: CommandResult): void => {
  const badge = requireElement<HTMLElement>("#result-badge");
  badge.textContent = result.success ? "执行成功" : "执行失败";
  badge.className = `status ${result.success ? "success" : "failure"}`;
  requireElement<HTMLElement>("#exit-code").textContent =
    result.exitCode === null ? "不可用" : String(result.exitCode);
  requireElement<HTMLElement>("#stdout").textContent = result.stdout || "（空）";
  requireElement<HTMLElement>("#stderr").textContent = result.stderr || "（空）";
  requireElement<HTMLDListElement>("#result").hidden = false;
};

window.addEventListener("DOMContentLoaded", () => {
  requireElement<HTMLButtonElement>("#refresh-button").addEventListener("click", () => void loadConfig());
  requireElement<HTMLButtonElement>("#add-button").addEventListener("click", () => openEditor());
  requireElement<HTMLButtonElement>("#cancel-button").addEventListener("click", closeEditor);
  requireElement<HTMLInputElement>("#autostart-toggle").addEventListener("change", (event) => {
    void changeAutostart((event.currentTarget as HTMLInputElement).checked);
  });
  requireElement<HTMLButtonElement>("#export-button").addEventListener("click", () => void exportItems());
  requireElement<HTMLButtonElement>("#import-button").addEventListener("click", () => void importItems());
  requireElement<HTMLInputElement>("#entry-icon").addEventListener("change", async (event) => {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      editorIcon = await loadIcon(file);
      renderIconPreview();
      requireElement<HTMLElement>("#form-error").textContent = "";
    } catch (error) {
      showMutationError(error);
    }
  });
  requireElement<HTMLButtonElement>("#remove-icon-button").addEventListener("click", () => {
    editorIcon = null;
    requireElement<HTMLInputElement>("#entry-icon").value = "";
    renderIconPreview();
  });
  requireElement<HTMLFormElement>("#entry-form").addEventListener("submit", (event) => {
    event.preventDefault();
    void saveEditor();
  });
  void listen<CommandResult>("command-finished", ({ payload }) => renderResult(payload));
  void loadConfig();
  void loadAutostart();
});
