import { create } from 'zustand'
import { EditSession, type TextLook } from '@/lib/EditSession.ts'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import { useToastStore } from '@/stores/toastStore.ts'
import { onToolChosen } from '@/stores/workspaceStore.ts'
import type { DocumentChange, Highlight, PageFont } from '@/types/protocol.ts'

export interface ActiveEdit {
  pageId: string
  caret: Highlight | null
  frame: Highlight
  selection: Highlight[]
  look: TextLook
}

interface CaretState {
  pageId: string
  line: number
  offset: number
}

interface EditingStore {
  enabled: boolean
  active: ActiveEdit | null
  fonts: PageFont[]
  session(): EditSession | null
  setEnabled(enabled: boolean): void
  begin(pageId: string, x: number, y: number): Promise<boolean>
  press(pageId: string, x: number, y: number, clicks: number, extend: boolean): Promise<boolean>
  dragTo(pageId: string, x: number, y: number): Promise<void>
  beginNew(pageId: string, x: number, y: number): Promise<void>
  edit(change: (session: EditSession) => EditSession): void
  refresh(): Promise<void>
  commit(): Promise<void>
  cancel(): Promise<void>
  undo(): Promise<void>
  redo(): Promise<void>
  reset(): void
}

function previewOf(session: EditSession) {
  const { pageId, line, draft, origin, newLook, text } = session
  if (!origin) return { kind: 'previewEdit', pageId, line, draft } as const
  const { caret, selection, dx, dy } = draft
  const family = newLook.font?.kind === 'bundled' ? newLook.font.family : 'sans'
  const catalog = newLook.font?.kind === 'catalog' ? newLook.font.key : null
  const { bold, italic, size } = newLook
  const color = newLook.color ?? [0, 0, 0]
  return { kind: 'previewNewText', pageId, draft: { x: origin[0] + dx, y: origin[1] + dy, text, caret, selection, family, catalog, bold, italic, size, color } } as const
}

function drawingOf(session: EditSession): string {
  const { text, box, styles, newLook } = session
  return JSON.stringify([text, box, styles, newLook])
}

const showNotice = (error: unknown) => useToastStore.getState().show('editing', (error as Error).message)

export const useEditingStore = create<EditingStore>()((set, get) => {
  let session: EditSession | null = null
  let drafted: EditSession | null = null
  let drawn = ''
  let previewedChange = false
  let sequence = 0

  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) showNotice(error)
    }
  }

  const hitDraft = async (pageId: string, x: number, y: number): Promise<number | null> => {
    try {
      return await engine.request({ kind: 'hitDraft', pageId, x, y }, { group: 'edit-hit' })
    } catch (error) {
      if (error instanceof CancelledError) return null
      throw error
    }
  }

  const loadFonts = async (pageId: string) => {
    const fonts = await engine.request({ kind: 'pageFonts', pageId })
    if (session?.pageId === pageId) set({ fonts })
  }

  const preview = async (next: EditSession) => {
    const previewing = ++sequence
    if (next.changed) previewedChange = true
    const { pageId } = next
    try {
      const { caret, frame, selection } = await engine.request(previewOf(next), { group: 'edit-preview' })
      if (previewing !== sequence) {
        if (session) drafted = next
        return
      }
      drafted = next
      const drawing = drawingOf(next)
      if (drawing !== drawn) {
        drawn = drawing
        useDocumentStore.getState().touchPages([pageId])
      }
      set({ active: { pageId, caret, frame, selection, look: next.look } })
    } catch (error) {
      if (error instanceof CancelledError || previewing !== sequence) return
      session = drafted
      showNotice(error)
      if (drafted) void preview(drafted)
    }
  }

  const start = async (next: EditSession) => {
    session = next
    drafted = next
    drawn = drawingOf(next)
    previewedChange = false
    await preview(next)
    await loadFonts(next.pageId)
  }

  const open = async (pageId: string, line: number, caret: (lineStart: number) => number, opening: number) => {
    const paragraph = await engine.request({ kind: 'paragraphText', pageId, line })
    if (opening !== sequence) return
    await start(EditSession.open(pageId, paragraph.line, paragraph.clusters, caret(paragraph.start), paragraph.styles))
  }

  const close = () => {
    sequence++
    session = null
    drafted = null
    previewedChange = false
    if (get().active) set({ active: null })
  }

  const redraw = (change: DocumentChange) => {
    const documents = useDocumentStore.getState()
    if (change.dirtyPages.length > 0) documents.touchPages(change.dirtyPages)
    documents.markChanged()
    void documents.refreshPages()
  }

  const revert = async (kind: 'undo' | 'redo') => {
    if (session) await get().cancel()
    const reverting = ++sequence
    await attempt(async () => {
      const change = await engine.request({ kind })
      redraw(change)
      if (change.uiState && get().enabled && reverting === sequence) {
        const { pageId, line, offset } = JSON.parse(change.uiState) as CaretState
        await open(pageId, line, () => offset, reverting)
      }
    })
  }

  return {
    enabled: false,
    active: null,
    fonts: [],

    session: () => session,

    setEnabled(enabled) {
      if (enabled === get().enabled) return
      if (!enabled) void get().commit()
      set({ enabled })
    },

    async begin(pageId, x, y) {
      void get().commit()
      const beginning = ++sequence
      let hit = false
      await attempt(async () => {
        const point = await engine.request({ kind: 'hitText', pageId, x, y })
        hit = point !== null
        if (point && beginning === sequence) await open(pageId, point.line, (lineStart) => lineStart + point.offset, beginning)
      })
      return hit
    },

    async press(pageId, x, y, clicks, extend) {
      if (session?.pageId !== pageId) return false
      const offset = await hitDraft(pageId, x, y)
      if (offset === null || session?.pageId !== pageId) return false
      get().edit((current) => {
        if (clicks >= 3) return current.selectAll()
        if (clicks === 2) return current.selectWord(offset)
        return extend ? current.extendTo(offset) : current.moveTo(offset)
      })
      return true
    },

    async dragTo(pageId, x, y) {
      if (session?.pageId !== pageId) return
      const offset = await hitDraft(pageId, x, y)
      if (offset !== null && session?.pageId === pageId) get().edit((current) => current.extendTo(offset))
    },

    async beginNew(pageId, x, y) {
      void get().commit()
      await start(EditSession.openNew(pageId, [x, y]))
    },

    edit(change) {
      if (!session) return
      const next = change(session)
      if (next === session) return
      session = next
      useToastStore.getState().clear('editing')
      void preview(next)
    },

    async refresh() {
      if (!session) return
      drawn = ''
      await preview(session)
    },

    async commit() {
      const current = session
      if (!current) return
      if (!current.changed) {
        await get().cancel()
        return
      }
      close()
      const caretState: CaretState = { pageId: current.pageId, line: current.line, offset: current.caret }
      const uiState = current.origin ? '' : JSON.stringify(caretState)
      await attempt(async () => redraw(await engine.request({ kind: 'commitEdit', uiState })))
    },

    async cancel() {
      const current = session
      if (!current) return
      const repaint = previewedChange
      close()
      await attempt(async () => {
        await engine.request({ kind: 'cancelEdit' })
        if (repaint) useDocumentStore.getState().touchPages([current.pageId])
      })
    },

    async undo() {
      if (session?.changed) {
        await get().cancel()
        return
      }
      await revert('undo')
    },

    async redo() {
      if (session?.changed) return
      await revert('redo')
    },

    reset() {
      close()
      set({ active: null })
      useToastStore.getState().clear('editing')
    },
  }
})

onDocumentReplaced(() => useEditingStore.getState().reset())

onToolChosen((tool) => useEditingStore.getState().setEnabled(tool === 'edit' || tool === 'addText'))
