export interface SelectionGesture {
  toggle: boolean
  range: boolean
}

export class PageSelection {
  static readonly EMPTY = new PageSelection(new Set(), null)

  readonly #selected: ReadonlySet<string>
  readonly anchor: string | null

  private constructor(selected: ReadonlySet<string>, anchor: string | null) {
    this.#selected = selected
    this.anchor = anchor
  }

  static of(pageIds: readonly string[]): PageSelection {
    return new PageSelection(new Set(pageIds), pageIds[pageIds.length - 1] ?? null)
  }

  get size(): number {
    return this.#selected.size
  }

  has(pageId: string): boolean {
    return this.#selected.has(pageId)
  }

  inOrder(order: readonly string[]): string[] {
    return order.filter((pageId) => this.#selected.has(pageId))
  }

  click(pageId: string, order: readonly string[], { toggle, range }: SelectionGesture): PageSelection {
    if (range && this.anchor !== null && order.includes(this.anchor)) {
      const [from, to] = [order.indexOf(this.anchor), order.indexOf(pageId)].sort((a, b) => a - b)
      return new PageSelection(new Set(order.slice(from, to + 1)), this.anchor)
    }
    if (toggle) {
      const selected = new Set(this.#selected)
      if (!selected.delete(pageId)) selected.add(pageId)
      return new PageSelection(selected, pageId)
    }
    return new PageSelection(new Set([pageId]), pageId)
  }

  within(order: readonly string[]): PageSelection {
    const kept = this.inOrder(order)
    if (kept.length === this.#selected.size) return this
    return new PageSelection(new Set(kept), this.anchor !== null && kept.includes(this.anchor) ? this.anchor : null)
  }
}
