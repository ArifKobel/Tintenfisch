import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { Box, DocumentChange, DocumentLink, Target } from '@engine/protocol.ts'
import type { TargetFollower } from '@navigation/TargetFollower.ts'
import type { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

export interface LinkChoice {
  pageId: string
  id: string
}

export interface LinkDraft {
  pageId: string
  linkId: string | null
  rect: Box
  target: Target | null
}

export interface LinksState {
  pages: Readonly<Record<string, readonly DocumentLink[]>>
  selected: LinkChoice | null
  draft: LinkDraft | null
  notice: string | null
}

type Listener = () => void

export class LinksStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #follower: TargetFollower
  readonly #listeners = new Set<Listener>()
  #state: LinksState = { pages: {}, selected: null, draft: null, notice: null }

  constructor(client: EngineClient, revisions: PageRevisionStore, follower: TargetFollower) {
    this.#client = client
    this.#revisions = revisions
    this.#follower = follower
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): LinksState => this.#state

  async load(pageId: string) {
    await this.#attempt(async () => {
      const links = await this.#client.request({ kind: 'links', pageId }, { group: `links-${pageId}` })
      const { selected } = this.#state
      const kept = selected?.pageId !== pageId || links.some((link) => link.id === selected.id) ? selected : null
      this.#update({ ...this.#state, pages: { ...this.#state.pages, [pageId]: links }, selected: kept })
    })
  }

  follow(link: DocumentLink) {
    this.#follower.follow(link.target)
  }

  select(choice: LinkChoice | null) {
    if (choice?.id !== this.#state.selected?.id) this.#update({ ...this.#state, selected: choice, notice: null })
  }

  draw(pageId: string, rect: Box) {
    this.#update({ ...this.#state, draft: { pageId, linkId: null, rect, target: null } })
  }

  editSelected() {
    const link = this.#selectedLink()
    const { selected } = this.#state
    if (link && selected) this.#update({ ...this.#state, draft: { pageId: selected.pageId, linkId: link.id, rect: link.rect, target: link.target } })
  }

  closeDraft() {
    if (this.#state.draft) this.#update({ ...this.#state, draft: null })
  }

  async save(target: Target) {
    const draft = this.#state.draft
    if (!draft) return
    await this.#attempt(async () => {
      const { pageId, linkId, rect } = draft
      if (linkId) {
        await this.#changed(pageId, await this.#client.request({ kind: 'updateLink', pageId, linkId, rect, target }))
        this.#update({ ...this.#state, draft: null })
      } else {
        const added = await this.#client.request({ kind: 'addLink', pageId, rect, target })
        await this.#changed(pageId, added)
        this.#update({ ...this.#state, draft: null, selected: { pageId, id: added.id } })
      }
    })
  }

  async move(choice: LinkChoice, rect: Box) {
    const link = this.#state.pages[choice.pageId]?.find((candidate) => candidate.id === choice.id)
    if (!link?.target) return
    const target = link.target
    await this.#attempt(async () => {
      await this.#changed(choice.pageId, await this.#client.request({ kind: 'updateLink', pageId: choice.pageId, linkId: choice.id, rect, target }))
    })
  }

  async removeSelected() {
    const selected = this.#state.selected
    if (!selected) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'deleteComment', pageId: selected.pageId, commentId: selected.id })
      this.#update({ ...this.#state, selected: null, draft: null })
      await this.#changed(selected.pageId, change)
    })
  }

  reset() {
    this.#update({ pages: {}, selected: null, draft: null, notice: null })
  }

  #selectedLink(): DocumentLink | null {
    const { selected, pages } = this.#state
    return (selected && pages[selected.pageId]?.find((link) => link.id === selected.id)) ?? null
  }

  async #changed(pageId: string, change: DocumentChange) {
    this.#revisions.touch(change.dirtyPages)
    await this.load(pageId)
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: LinksState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
