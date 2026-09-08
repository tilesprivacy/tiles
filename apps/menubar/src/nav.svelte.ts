export type ViewId = "root" | "account" | "sessions" | "model";

/** views are pushed onto the root, never replacing it */
class Nav {
  stack = $state<ViewId[]>(["root"]);

  get top(): ViewId {
    return this.stack[this.stack.length - 1];
  }

  get depth(): number {
    return this.stack.length - 1;
  }

  push(id: ViewId): void {
    // a double click on a row would otherwise stack the same view twice
    if (this.top === id) return;
    this.stack = [...this.stack, id];
  }

  pop(): void {
    if (this.stack.length > 1) this.stack = this.stack.slice(0, -1);
  }
}

export const nav = new Nav();
