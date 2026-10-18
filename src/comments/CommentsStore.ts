import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { Comment, MarkupRequest } from '@engine/protocol.ts'
import type { ToolStore } from '@tools/ToolStore.ts'
import type { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

const STROKE_WIDTH = 2

export interface CommentsState {
  comments: Comment[]
  selected: string | null
  open: boolean
  notice: string | null
}

type Listener = () => void

export class CommentsStore {
  readonly #client: EngineClient
  readonly #tools: ToolStore
  readonly #revisions: PageRevisionStore
  readonly #now: () => Date
  readonly #listeners = new Set<Listener>()
  #state: CommentsState = { comments: [], selected: null, open: false, notice: null }
  #sequence = 0

  constructor(client: EngineClient, tools: ToolStore, revisions: PageRevisionStore, now: () => Date = () => new Date()) {
    this.#client = client
    this.#tools = tools
    this.#revisions = revisions
    this.#now = now
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): CommentsState => this.#state

  static pdfDate(date: Date): string {
    const two = (value: number) => String(value).padStart(2, '0')
    return `D:${date.getUTCFullYear()}${two(date.getUTCMonth() + 1)}${two(date.getUTCDate())}${two(date.getUTCHours())}${two(date.getUTCMinutes())}${two(date.getUTCSeconds())}Z`
  }

  async reload() {
    const sequence = ++this.#sequence
    await this.#attempt(async () => {
      const comments = await this.#client.request({ kind: 'comments' }, { group: 'comments' })
      if (sequence !== this.#sequence) return
      const selected = comments.some((comment) => comment.id === this.#state.selected) ? this.#state.selected : null
      this.#update({ ...this.#state, comments, selected })
    })
  }

  async add(pageId: string, markup: MarkupRequest) {
    const style = { color: this.#tools.activeColor, width: STROKE_WIDTH, author: '', date: CommentsStore.pdfDate(this.#now()) }
    await this.#attempt(async () => {
      const added = await this.#client.request({ kind: 'addMarkup', pageId, markup, style })
      this.#revisions.touch(added.dirtyPages)
      await this.reload()
      const opens = markup.kind === 'note' || markup.kind === 'textBox'
      this.#update({ ...this.#state, selected: added.id, open: this.#state.open || opens })
    })
  }

  async remove(id: string) {
    const comment = this.#state.comments.find((candidate) => candidate.id === id)
    if (!comment) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'deleteComment', pageId: comment.pageId, commentId: id })
      this.#revisions.touch(change.dirtyPages)
      await this.reload()
    })
  }

  async removeSelected() {
    if (this.#state.selected) await this.remove(this.#state.selected)
  }

  async setContents(id: string, contents: string) {
    const comment = this.#state.comments.find((candidate) => candidate.id === id)
    if (!comment || comment.contents === contents) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'setCommentContents', commentId: id, contents, date: CommentsStore.pdfDate(this.#now()) })
      this.#revisions.touch(change.dirtyPages)
      await this.reload()
    })
  }

  select(id: string | null) {
    if (id !== this.#state.selected) this.#update({ ...this.#state, selected: id })
  }

  togglePanel() {
    this.#update({ ...this.#state, open: !this.#state.open })
  }

  reset() {
    this.#sequence++
    this.#update({ ...this.#state, comments: [], selected: null, notice: null })
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: CommentsState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
