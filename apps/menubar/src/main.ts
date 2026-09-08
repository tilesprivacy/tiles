import { invoke } from "@tauri-apps/api/core";
import { mount } from "svelte";

import Panel from "./Panel.svelte";
import "./styles/base.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("#app is declared in index.html");
}

mount(Panel, { target });

/** swallows errors so the page still runs in a plain browser */
function tell(command: string): void {
  void invoke(command).catch(() => {});
}

// load and layout fire while the webview is still blank, the nested rAF means
// a full frame has been through. the host holds the panel hidden until then
requestAnimationFrame(() => {
  requestAnimationFrame(() => tell("panel_ready"));
});

window.addEventListener("contextmenu", (event) => event.preventDefault());
