import { invoke } from "@tauri-apps/api/core";

const COPIED_FOR = 1200;

/** the copy button's whole state, so both callers get the honest version */
export class Copier {
  copied = $state(false);
  #timer = 0;

  /** saying Copied when nothing reached the pasteboard is worse than silence */
  async copy(text: string): Promise<void> {
    try {
      await invoke("copy_text", { text });
    } catch (err) {
      console.error("[copy]", err);
      return;
    }

    this.copied = true;
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => (this.copied = false), COPIED_FOR);
  }

  dispose(): void {
    clearTimeout(this.#timer);
  }
}
