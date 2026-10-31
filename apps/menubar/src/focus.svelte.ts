/** the row the rail is sitting on, shared by the pointer and the arrow keys */
class Focus {
  active = $state<HTMLElement | null>(null);

  /** kept in document order, so no view has to number its own rows */
  #rows: HTMLElement[] = [];
  #run = new WeakMap<HTMLElement, () => void>();

  register(row: HTMLElement, run: () => void): () => void {
    this.#run.set(row, run);
    const at = this.#rows.findIndex(
      (other) => other.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_PRECEDING,
    );
    this.#rows.splice(at === -1 ? this.#rows.length : at, 0, row);

    return () => {
      this.#rows = this.#rows.filter((other) => other !== row);
      if (this.active === row) this.active = null;
    };
  }

  /** a push leaves both views mounted for the length of the animation */
  reset(): void {
    this.active = null;
  }

  move(step: 1 | -1): void {
    if (this.#rows.length === 0) return;
    const from = this.active ? this.#rows.indexOf(this.active) : -1;
    const to = from === -1 ? (step === 1 ? 0 : this.#rows.length - 1) : from + step;
    this.active = this.#rows.at(to % this.#rows.length) ?? null;
  }

  activate(): void {
    if (this.active) this.#run.get(this.active)?.();
  }
}

export const focus = new Focus();
