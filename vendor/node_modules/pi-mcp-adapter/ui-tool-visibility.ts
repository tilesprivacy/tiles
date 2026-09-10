export type UiToolVisibility = "model" | "app";

export function extractUiToolVisibility(meta: Record<string, unknown> | undefined): UiToolVisibility[] | undefined {
  if (!meta || typeof meta !== "object") return undefined;
  const ui = meta.ui;
  if (!ui || typeof ui !== "object" || Array.isArray(ui)) return undefined;
  const visibility = (ui as Record<string, unknown>).visibility;
  if (visibility === undefined) return undefined;
  if (!Array.isArray(visibility)) return [];

  const values: UiToolVisibility[] = [];
  for (const entry of visibility) {
    if (entry !== "model" && entry !== "app") return [];
    if (!values.includes(entry)) values.push(entry);
  }
  return values;
}

export function isUiToolVisibleToModel(visibility: readonly UiToolVisibility[] | undefined): boolean {
  return visibility === undefined || visibility.includes("model");
}

export function isUiToolCallableByApp(visibility: readonly UiToolVisibility[] | undefined): boolean {
  return visibility === undefined || visibility.includes("app");
}
