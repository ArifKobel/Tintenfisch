import type { DocumentStore } from '@/stores/DocumentStore.ts'
import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { PagesChange, PageSummary } from '@/types/protocol.ts'
import { PageSelection, type SelectionGesture } from '@/lib/PageSelection.ts'
import type { PageRevisionStore } from '@/stores/PageRevisionStore.ts'

export interface OrganizeState {
  selection: PageSelection
  notice: string | null
  lockedInsert: { name: string; wrongPassword: boolean } | null
}

export interface ExtractedPages {
  name: string
  bytes: Uint8Array
}

type Listener = () => void

const INITIAL: OrganizeState = { selection: PageSelection.EMPTY, notice: null, lockedInsert: null }

export class OrganizeStore {
  readonly #client: EngineClient
  readonly #documents: DocumentStore
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  readonly #changeListeners = new Set<Listener>()
  #state: OrganizeState = INITIAL
  #pendingInsert: { bytes: Uint8Array; before: string | null } | null = null

  constructor(client: EngineClient, documents: DocumentStore, revisions: PageRevisionStore) {
    this.#client = client
    this.#documents = documents
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): OrganizeState => this.#state

  onDocumentChange(listener: Listener): () => void {
    this.#changeListeners.add(listener)
    return () => this.#changeListeners.delete(listener)
  }

  select(pageId: string, gesture: SelectionGesture) {
    this.#update({ ...this.#state, selection: this.#state.selection.click(pageId, this.#order(), gesture) })
  }

  async rotate(quarterTurns: number) {
    const pageIds = this.#selected()
    if (pageIds.length === 0) return
    await this.#attempt(async () => this.#apply(await this.#client.request({ kind: 'rotatePages', pageIds, quarterTurns })))
  }

  async deleteSelected() {
    const pageIds = this.#selected()
    if (pageIds.length === 0) return
    await this.#attempt(async () => this.#apply(await this.#client.request({ kind: 'deletePages', pageIds })))
  }

  async move(pageIds: string[], before: string | null) {
    if (pageIds.length === 0) return
    await this.#attempt(async () => this.#apply(await this.#client.request({ kind: 'movePages', pageIds, before })))
  }

  async insertBlank() {
    const size = this.#pageAt(this.#selected().at(-1)) ?? this.#pages()[0]
    const before = this.#insertionPoint()
    await this.#attempt(async () =>
      this.#apply(
        await this.#client.request({ kind: 'insertBlankPage', before, width: size?.width ?? 595, height: size?.height ?? 842 }),
      ),
    )
  }

  async insertFile(name: string, bytes: Uint8Array) {
    this.#pendingInsert = { bytes, before: this.#insertionPoint() }
    await this.#insert(name, '')
  }

  async unlockInsert(password: string) {
    const locked = this.#state.lockedInsert
    if (locked) await this.#insert(locked.name, password)
  }

  cancelInsert() {
    this.#pendingInsert = null
    this.#update({ ...this.#state, lockedInsert: null })
  }

  async extract(): Promise<ExtractedPages | null> {
    const pageIds = this.#selected()
    const document = this.#documents.getSnapshot()
    if (pageIds.length === 0 || document.phase !== 'ready') return null
    try {
      const bytes = await this.#client.request({ kind: 'extractPages', pageIds })
      return { name: `${document.name.replace(/\.pdf$/i, '')} (pages).pdf`, bytes }
    } catch (error) {
      this.#fail(error)
      return null
    }
  }

  reset() {
    this.#pendingInsert = null
    this.#update(INITIAL)
  }

  async #insert(name: string, password: string) {
    const pending = this.#pendingInsert
    if (!pending) return
    await this.#attempt(async () => {
      const result = await this.#client.request({ kind: 'insertDocument', bytes: pending.bytes.slice(), password, before: pending.before })
      if (result.status === 'locked') {
        this.#update({ ...this.#state, lockedInsert: { name, wrongPassword: result.wrongPassword } })
        return
      }
      this.#pendingInsert = null
      this.#documents.replacePages(result.pages)
      this.#update({ ...this.#state, selection: PageSelection.of(result.inserted), lockedInsert: null, notice: null })
      this.#changeListeners.forEach((listener) => listener())
    })
  }

  #apply(change: PagesChange) {
    this.#documents.replacePages(change.pages)
    if (change.dirtyPages.length > 0) this.#revisions.touch(change.dirtyPages)
    this.#update({ ...this.#state, selection: this.#state.selection.within(this.#order()), notice: null })
    this.#changeListeners.forEach((listener) => listener())
  }

  #pages(): PageSummary[] {
    const document = this.#documents.getSnapshot()
    return document.phase === 'ready' ? document.pages : []
  }

  #order(): string[] {
    return this.#pages().map((page) => page.id)
  }

  #selected(): string[] {
    return this.#state.selection.inOrder(this.#order())
  }

  #pageAt(pageId: string | undefined): PageSummary | undefined {
    return this.#pages().find((page) => page.id === pageId)
  }

  #insertionPoint(): string | null {
    const order = this.#order()
    const last = this.#selected().at(-1)
    if (last === undefined) return null
    return order[order.indexOf(last) + 1] ?? null
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      this.#fail(error)
    }
  }

  #fail(error: unknown) {
    if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
  }

  #update(state: OrganizeState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
