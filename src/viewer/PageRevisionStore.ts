type Listener = () => void

export class PageRevisionStore {
  readonly #listeners = new Set<Listener>()
  #revisions = new Map<string, number>()

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  revisionOf(pageId: string): number {
    return this.#revisions.get(pageId) ?? 0
  }

  touch(pageIds: Iterable<string>) {
    const revisions = new Map(this.#revisions)
    for (const pageId of pageIds) revisions.set(pageId, (revisions.get(pageId) ?? 0) + 1)
    this.#revisions = revisions
    this.#listeners.forEach((listener) => listener())
  }
}
