import { invoke } from "@tauri-apps/api/core";

/**
 * A command sitting behind a button. Swallowing the error is what makes a dead
 * button look like a working one, so a failure at least reaches the console.
 */
export function act(command: string, args?: Record<string, unknown>): void {
  void invoke(command, args).catch((err) => console.error(`[${command}]`, err));
}
