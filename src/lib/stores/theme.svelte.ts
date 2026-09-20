export type ThemeMode = "system" | "light" | "dark";

const MODE_KEY = "tasktimer-theme-mode";
const ACCENT_KEY = "tasktimer-accent";
const ACCENT_SOFT_KEY = "tasktimer-accent-soft";
const OPACITY_KEY = "tasktimer-window-opacity";

class ThemeStore {
  mode = $state<ThemeMode>("system");
  accent = $state<string | null>(null);
  accentSoft = $state<string | null>(null);
  opacity = $state(80);

  load() {
    const storedMode = localStorage.getItem(MODE_KEY);
    if (storedMode === "light" || storedMode === "dark" || storedMode === "system") this.mode = storedMode;
    this.accent = localStorage.getItem(ACCENT_KEY);
    this.accentSoft = localStorage.getItem(ACCENT_SOFT_KEY);
    const storedOpacity = Number(localStorage.getItem(OPACITY_KEY));
    if (storedOpacity >= 10 && storedOpacity <= 100) this.opacity = storedOpacity;
    this.apply();
  }

  setOpacity(opacity: number) {
    this.opacity = opacity;
    localStorage.setItem(OPACITY_KEY, String(opacity));
    this.apply();
  }

  setMode(mode: ThemeMode) {
    this.mode = mode;
    localStorage.setItem(MODE_KEY, mode);
    this.apply();
  }

  setAccent(accent: string | null, accentSoft: string | null) {
    this.accent = accent;
    this.accentSoft = accentSoft;
    if (accent && accentSoft) {
      localStorage.setItem(ACCENT_KEY, accent);
      localStorage.setItem(ACCENT_SOFT_KEY, accentSoft);
    } else {
      localStorage.removeItem(ACCENT_KEY);
      localStorage.removeItem(ACCENT_SOFT_KEY);
    }
    this.apply();
  }

  private apply() {
    const root = document.documentElement;
    if (this.mode === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", this.mode);
    if (this.accent) root.style.setProperty("--accent", this.accent);
    else root.style.removeProperty("--accent");
    if (this.accentSoft) root.style.setProperty("--accent-soft", this.accentSoft);
    else root.style.removeProperty("--accent-soft");
    root.style.setProperty("--window-opacity-pct", `${this.opacity}%`);
  }
}

export const themeStore = new ThemeStore();
