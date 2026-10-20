import { EditSession, type TextLook } from '@editing/EditSession.ts'
import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { DocumentChange, Highlight, PageFont } from '@engine/protocol.ts'
import type { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

export interface EditingState {
  enabled: boolean
  active: { pageId: string; caret: Highlight | null; frame: Highlight; selection: Highlight[]; look: TextLook } | null
  fonts: PageFont[]
  notice: string | null
}

interface CaretState {
  pageId: string
  line: number
  offset: number
}

type Listener = () => void

export class EditingStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  readonly #changeListeners = new Set<Listener>()
  #state: EditingState = { enabled: false, active: null, fonts: [], notice: null }
  #session: EditSession | null = null
  #drafted: EditSession | null = null
  #drawn = ''
  #previewedChange = false
  #sequence = 0

  constructor(client: EngineClient, revisions: PageRevisionStore) {
    this.#client = client
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): EditingState => this.#state

  get session(): EditSession | null {
    return this.#session
  }

  onDocumentChange(listener: Listener): () => void {
    this.#changeListeners.add(listener)
    return () => this.#changeListeners.delete(listener)
  }

  setEnabled(enabled: boolean) {
    if (enabled === this.#state.enabled) return
    if (!enabled) void this.commit()
    this.#update({ ...this.#state, enabled })
  }

  async begin(pageId: string, x: number, y: number): Promise<boolean> {
    void this.commit()
    const sequence = ++this.#sequence
    let hit = false
    await this.#attempt(async () => {
      const point = await this.#client.request({ kind: 'hitText', pageId, x, y })
      hit = point !== null
      if (point && sequence === this.#sequence) await this.#open(pageId, point.line, (start) => start + point.offset, sequence)
    })
    return hit
  }

  async press(pageId: string, x: number, y: number, clicks: number, extend: boolean): Promise<boolean> {
    if (this.#session?.pageId !== pageId) return false
    const offset = await this.#hitDraft(pageId, x, y)
    if (offset === null || this.#session?.pageId !== pageId) return false
    this.edit((session) => {
      if (clicks >= 3) return session.selectAll()
      if (clicks === 2) return session.selectWord(offset)
      return extend ? session.extendTo(offset) : session.moveTo(offset)
    })
    return true
  }

  async dragTo(pageId: string, x: number, y: number) {
    if (this.#session?.pageId !== pageId) return
    const offset = await this.#hitDraft(pageId, x, y)
    if (offset !== null && this.#session?.pageId === pageId) this.edit((session) => session.extendTo(offset))
  }

  async #hitDraft(pageId: string, x: number, y: number): Promise<number | null> {
    try {
      return await this.#client.request({ kind: 'hitDraft', pageId, x, y }, { group: 'edit-hit' })
    } catch (error) {
      if (error instanceof CancelledError) return null
      throw error
    }
  }

  async beginNew(pageId: string, x: number, y: number) {
    void this.commit()
    const session = EditSession.openNew(pageId, [x, y])
    this.#session = session
    this.#drafted = session
    this.#drawn = EditingStore.#drawingOf(session)
    this.#previewedChange = false
    await this.#preview(session)
    await this.#loadFonts(pageId)
  }

  edit(change: (session: EditSession) => EditSession) {
    const session = this.#session
    if (!session) return
    const next = change(session)
    if (next === session) return
    this.#session = next
    if (this.#state.notice) this.#update({ ...this.#state, notice: null })
    void this.#preview(next)
  }

  async refresh() {
    const session = this.#session
    if (!session) return
    this.#drawn = ''
    await this.#preview(session)
  }

  async commit() {
    const session = this.#session
    if (!session) return
    if (!session.changed) {
      await this.cancel()
      return
    }
    this.#close()
    const caretState: CaretState = { pageId: session.pageId, line: session.line, offset: session.caret }
    const uiState = session.origin ? '' : JSON.stringify(caretState)
    await this.#attempt(async () => this.#redraw(await this.#client.request({ kind: 'commitEdit', uiState })))
  }

  async cancel() {
    const session = this.#session
    if (!session) return
    const redraw = this.#previewedChange
    this.#close()
    await this.#attempt(async () => {
      await this.#client.request({ kind: 'cancelEdit' })
      if (redraw) this.#revisions.touch([session.pageId])
    })
  }

  async undo() {
    if (this.#session?.changed) {
      await this.cancel()
      return
    }
    await this.#revert('undo')
  }

  async redo() {
    if (this.#session?.changed) return
    await this.#revert('redo')
  }

  reset() {
    this.#close()
    this.#update({ ...this.#state, active: null, notice: null })
  }

  async #revert(kind: 'undo' | 'redo') {
    if (this.#session) await this.cancel()
    const sequence = ++this.#sequence
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind })
      this.#redraw(change)
      if (change.uiState && this.#state.enabled && sequence === this.#sequence) {
        const { pageId, line, offset } = JSON.parse(change.uiState) as CaretState
        await this.#open(pageId, line, () => offset, sequence)
      }
    })
  }

  async #open(pageId: string, line: number, caret: (lineStart: number) => number, sequence: number) {
    const paragraph = await this.#client.request({ kind: 'paragraphText', pageId, line })
    if (sequence !== this.#sequence) return
    const session = EditSession.open(pageId, paragraph.line, paragraph.clusters, caret(paragraph.start), paragraph.styles)
    this.#session = session
    this.#drafted = session
    this.#drawn = EditingStore.#drawingOf(session)
    this.#previewedChange = false
    await this.#preview(session)
    await this.#loadFonts(pageId)
  }

  async #loadFonts(pageId: string) {
    const fonts = await this.#client.request({ kind: 'pageFonts', pageId })
    if (this.#session?.pageId === pageId) this.#update({ ...this.#state, fonts })
  }

  async #preview(session: EditSession) {
    const sequence = ++this.#sequence
    if (session.changed) this.#previewedChange = true
    const { pageId } = session
    try {
      const { caret, frame, selection } = await this.#client.request(EditingStore.#previewOf(session), { group: 'edit-preview' })
      if (sequence !== this.#sequence) {
        if (this.#session) this.#drafted = session
        return
      }
      this.#drafted = session
      const drawing = EditingStore.#drawingOf(session)
      if (drawing !== this.#drawn) {
        this.#drawn = drawing
        this.#revisions.touch([pageId])
      }
      this.#update({ ...this.#state, active: { pageId, caret, frame, selection, look: session.look } })
    } catch (error) {
      if (error instanceof CancelledError || sequence !== this.#sequence) return
      this.#session = this.#drafted
      this.#update({ ...this.#state, notice: (error as Error).message })
      if (this.#drafted) void this.#preview(this.#drafted)
    }
  }

  static #previewOf(session: EditSession) {
    const { pageId, line, draft, origin, newLook, text } = session
    if (!origin) return { kind: 'previewEdit', pageId, line, draft } as const
    const { caret, selection, dx, dy } = draft
    const family = newLook.font?.kind === 'bundled' ? newLook.font.family : 'sans'
    const catalog = newLook.font?.kind === 'catalog' ? newLook.font.key : null
    const { bold, italic, size } = newLook
    const color = newLook.color ?? [0, 0, 0]
    return { kind: 'previewNewText', pageId, draft: { x: origin[0] + dx, y: origin[1] + dy, text, caret, selection, family, catalog, bold, italic, size, color } } as const
  }

  static #drawingOf(session: EditSession): string {
    const { text, box, styles, newLook } = session
    return JSON.stringify([text, box, styles, newLook])
  }

  #redraw(change: DocumentChange) {
    if (change.dirtyPages.length > 0) this.#revisions.touch(change.dirtyPages)
    this.#changeListeners.forEach((listener) => listener())
  }

  #close() {
    this.#sequence++
    this.#session = null
    this.#drafted = null
    this.#previewedChange = false
    if (this.#state.active) this.#update({ ...this.#state, active: null })
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: EditingState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
