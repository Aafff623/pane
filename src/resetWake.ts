import { invoke } from "@tauri-apps/api/core";
import { t } from "./i18n";

type Settings = { zai: boolean; minimax: boolean; zaiRegion: "global" | "cn"; minimaxRegion: "global" | "cn" };
export type WakeStatus = { provider: string; scheduledResetAt: number | null; attemptedResetAt: number | null; attemptedAt: number | null; status: string; message: string };
let settings: Settings = { zai: false, minimax: false, zaiRegion: "global", minimaxRegion: "global" };
let statuses: WakeStatus[] = [];

export function acceptWakeStatuses(next: WakeStatus[]): void { statuses = next; }

export async function loadResetWake(): Promise<void> {
  [settings, statuses] = await Promise.all([
    invoke<Settings>("reset_wake_settings"), invoke<WakeStatus[]>("reset_wake_status"),
  ]);
}

export function resetWakeBadge(provider: string): string {
  if ((provider !== "zai" && provider !== "minimax") || !settings[provider]) return "";
  const status = statuses.find((row) => row.provider === provider);
  const state = status?.status || "waiting";
  const line = document.createElement("p");
  line.className = "reset-wake-status";
  line.textContent = t(`settings.wake.${["success", "failed", "attempted", "skipped"].includes(state) ? state : "waiting"}`);
  if (status?.attemptedAt) line.title = new Date(status.attemptedAt).toLocaleString();
  if (state === "failed" && status?.message) line.title += ` · ${status.message}`;
  return line.outerHTML;
}

export async function mountResetWakeSettings(
  root: HTMLElement,
  confirm: () => Promise<boolean>,
  onChange: () => void,
): Promise<void> {
  root.replaceChildren();
  const heading = document.createElement("strong"); heading.textContent = t("settings.wake.title");
  heading.dataset.i18n = "settings.wake.title";
  const help = document.createElement("p"); help.textContent = t("settings.wake.description");
  help.dataset.i18n = "settings.wake.description";
  const feedback = document.createElement("p"); feedback.setAttribute("role", "status");
  root.append(heading, help, feedback);
  try { await loadResetWake(); } catch { feedback.textContent = t("settings.wake.loadFailed"); return; }
  let busy = false;
  const controls: { id: "zai" | "minimax"; toggle: HTMLInputElement; region: HTMLSelectElement }[] = [];
  for (const id of ["zai", "minimax"] as const) {
    const label = document.createElement("label"); label.className = "wake-setting-row";
    const toggle = document.createElement("input"); toggle.type = "checkbox"; toggle.checked = settings[id];
    toggle.setAttribute("aria-label", t(`settings.wake.${id}`));
    toggle.dataset.i18nAria = `settings.wake.${id}`;
    const name = document.createElement("span"); name.textContent = t(`settings.wake.${id}`);
    const region = document.createElement("select"); region.setAttribute("aria-label", `${name.textContent} · ${t("settings.wake.region")}`);
    for (const value of ["global", "cn"] as const) {
      const option = document.createElement("option"); option.value = value;
      option.textContent = t(`settings.wake.${value}`);
      option.dataset.i18n = `settings.wake.${value}`;
      region.append(option);
    }
    region.value = settings[`${id}Region`];
    controls.push({ id, toggle, region });
    const change = async () => {
      if (busy) return;
      busy = true;
      for (const control of controls) { control.toggle.disabled = true; control.region.disabled = true; }
      try {
        const next = { ...settings, [id]: toggle.checked, [`${id}Region`]: region.value } as Settings;
        const needsConsent = next[id] && (!settings[id] || next[`${id}Region`] !== settings[`${id}Region`]);
        if (needsConsent && !(await confirm())) return;
        await invoke("reset_wake_set", { settings: next, consent: needsConsent });
        settings = next;
        feedback.textContent = t("settings.wake.saved");
        onChange();
      } catch { feedback.textContent = t("settings.wake.saveFailed"); }
      finally {
        busy = false;
        for (const control of controls) {
          control.toggle.disabled = false; control.region.disabled = false;
          control.toggle.checked = settings[control.id]; control.region.value = settings[`${control.id}Region`];
        }
      }
    };
    toggle.addEventListener("change", () => void change());
    region.addEventListener("change", () => void change());
    label.append(toggle, name, region); root.insertBefore(label, feedback);
  }
}
