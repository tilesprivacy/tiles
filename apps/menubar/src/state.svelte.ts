import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Health =
  | { state: "down"; reason: string }
  | { state: "starting" }
  | { state: "up"; version: string };

export type Power = "unknown" | "off" | "starting" | "on";

/** the `[llama]` table, every flag optional in the daemon too */
export type Llama = {
  contextLength: number | null;
  gpuLayers: number | null;
  offloadKqv: boolean | null;
  batchSize: number | null;
  mtp: boolean | null;
  nCpuMoe: number | null;
  flashAttn: boolean | null;
  noMmap: boolean | null;
};

export type Inference = { power: Power; model: string | null; llama: Llama | null };

export type Account =
  | { state: "unknown" }
  | { state: "none" }
  | { state: "local"; did: string; nickname: string };

export type Session = { id: string; name: string; createdAt: number };
export type Sessions = { state: "unknown" } | { state: "ready"; sessions: Session[] };

export type Remote =
  | { state: "unknown" }
  | { state: "off" }
  | { state: "sharing"; ticket: string };

export const health = $state<{ value: Health }>({ value: { state: "starting" } });
export const inference = $state<{ value: Inference }>({
  value: { power: "unknown", model: null, llama: null },
});
export const account = $state<{ value: Account }>({ value: { state: "unknown" } });
export const sessions = $state<{ value: Sessions }>({ value: { state: "unknown" } });
export const remote = $state<{ value: Remote }>({ value: { state: "unknown" } });

/**
 * events only fire on a change, so every state has to be asked for once as
 * well. returns the teardown for all five listeners
 */
export function connect(): () => void {
  let connected = true;

  const sync = <T>(event: string, command: string, apply: (value: T) => void) => {
    let generation = 0;
    return listen<T>(event, ({ payload }) => {
      generation += 1;
      if (connected) apply(payload);
    })
      .then((unlisten) => {
        if (!connected) {
          unlisten();
          return () => {};
        }

        const snapshotGeneration = generation;
        void invoke<T>(command)
          .then((value) => {
            if (connected && generation === snapshotGeneration) apply(value);
          })
          .catch(() => {});
        return unlisten;
      })
      .catch(() => {
        // A snapshot is still useful if event registration itself failed.
        if (connected) {
          void invoke<T>(command)
            .then((value) => connected && apply(value))
            .catch(() => {});
        }
        return () => {};
      });
  };

  const listeners: Promise<UnlistenFn>[] = [
    sync<Health>("daemon://health", "daemon_health", (v) => (health.value = v)),
    sync<Inference>("inference://state", "inference_state", (v) => (inference.value = v)),
    sync<Account>("account://state", "account_state", (v) => (account.value = v)),
    sync<Sessions>("sessions://state", "sessions_state", (v) => (sessions.value = v)),
    sync<Remote>("remote://state", "remote_state", (v) => (remote.value = v)),
  ];

  return () => {
    connected = false;
    listeners.forEach((listener) => void listener.then((unlisten) => unlisten()));
  };
}

/** both ends carry the meaning, the middle is a base32 blur at 380px */
export function truncateMiddle(text: string, head: number, tail: number): string {
  return text.length <= head + tail + 2 ? text : `${text.slice(0, head)}…${text.slice(-tail)}`;
}
