import type { DocumentStore } from '@document/DocumentStore.ts'
import type { AutoSave } from '@files/AutoSave.ts'
import type { DocumentLibrary, StoredFile } from '@files/DocumentLibrary.ts'
import type { Router } from '@files/Router.ts'

export interface FilesState {
  files: StoredFile[]
  current: string | null
  missing: boolean
  deleting: StoredFile | null
  notice: string | null
}

type Listener = () => void

export class FilesStore {
  readonly #library: DocumentLibrary
  readonly #documents: DocumentStore
  readonly #router: Router
  readonly #autoSave: AutoSave
  readonly #listeners = new Set<Listener>()
  #state: FilesState = { files: [], current: null, missing: false, deleting: null, notice: null }

  constructor(library: DocumentLibrary, documents: DocumentStore, router: Router, autoSave: AutoSave) {
    this.#library = library
    this.#documents = documents
    this.#router = router
    this.#autoSave = autoSave
    documents.onDocumentReplaced(() => autoSave.track(documents.getSnapshot().phase === 'ready' ? this.#state.current : null))
    router.subscribe(() => void this.#follow())
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): FilesState => this.#state

  async start() {
    await Promise.all([this.refresh(), this.#follow()])
  }

  async refresh() {
    await this.#attempt(async () => this.#update({ ...this.#state, files: await this.#library.list() }))
  }

  async openNew(name: string, bytes: Uint8Array) {
    await this.#leave()
    await this.#attempt(async () => {
      const file = await this.#library.add(name, bytes)
      void globalThis.navigator?.storage?.persist?.()
      this.#update({ ...this.#state, current: file.id, missing: false, notice: null })
      this.#router.go({ kind: 'document', id: file.id })
      await this.#documents.open(name, bytes)
    })
  }

  show(id: string) {
    this.#router.go({ kind: 'document', id })
  }

  report(notice: string) {
    this.#update({ ...this.#state, notice })
  }

  goHome() {
    this.#router.go({ kind: 'home' })
  }

  askToDelete(file: StoredFile | null) {
    this.#update({ ...this.#state, deleting: file })
  }

  async confirmDelete() {
    const file = this.#state.deleting
    if (!file) return
    this.#update({ ...this.#state, deleting: null })
    await this.#attempt(async () => {
      if (file.id === this.#state.current) {
        await this.#leave()
        this.#router.replace({ kind: 'home' })
      }
      await this.#library.remove(file.id)
      await this.refresh()
    })
  }

  async #follow() {
    const route = this.#router.getSnapshot()
    if (route.kind === 'home') {
      await this.#leave()
      this.#documents.close()
      await this.refresh()
      return
    }
    if (route.id === this.#state.current) return
    await this.#leave()
    await this.#attempt(async () => {
      const stored = await this.#library.open(route.id)
      if (!stored) {
        this.#documents.close()
        this.#update({ ...this.#state, missing: true })
        this.#router.replace({ kind: 'home' })
        return
      }
      this.#update({ ...this.#state, current: route.id, missing: false })
      await this.#documents.open(stored.file.name, stored.bytes)
    })
  }

  async #leave() {
    await this.#autoSave.flush()
    this.#autoSave.track(null)
    this.#update({ ...this.#state, current: null })
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: FilesState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
