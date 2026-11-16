export interface Toast { message: string; actionLabel?: string; action?: () => void | Promise<void> }

/** One transient message at a time, shown by the layout. Survives the list that raised it re-rendering. */
class ToastStore {
  current = $state<Toast | null>(null);
  private handle: ReturnType<typeof setTimeout> | null = null;

  show(toast: Toast, durationMs = 6000) {
    if (this.handle !== null) clearTimeout(this.handle);
    this.current = toast;
    this.handle = setTimeout(() => this.dismiss(), durationMs);
  }

  dismiss() {
    if (this.handle !== null) clearTimeout(this.handle);
    this.handle = null;
    this.current = null;
  }

  async runAction() {
    const action = this.current?.action;
    this.dismiss();
    await action?.();
  }
}

export const toastStore = new ToastStore();
