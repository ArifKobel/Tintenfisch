import { create } from 'zustand'
import { pdfDate } from '@/lib/pdfDate.ts'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentChanged, onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import type { Comment, MarkupRequest } from '@/types/protocol.ts'

const STROKE_WIDTH = 2

interface CommentsStore {
  comments: Comment[]
  selected: string | null
  notice: string | null
  reload(): Promise<void>
  add(pageId: string, markup: MarkupRequest): Promise<void>
  remove(id: string): Promise<void>
  removeSelected(): Promise<void>
  setContents(id: string, contents: string): Promise<void>
  select(id: string | null): void
  reset(): void
}

export const useCommentsStore = create<CommentsStore>()((set, get) => {
  let sequence = 0

  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
    }
  }

  const commentOf = (id: string) => get().comments.find((comment) => comment.id === id)

  return {
    comments: [],
    selected: null,
    notice: null,

    async reload() {
      const loading = ++sequence
      await attempt(async () => {
        const comments = await engine.request({ kind: 'comments' }, { group: 'comments' })
        if (loading !== sequence) return
        const { selected } = get()
        set({ comments, selected: comments.some((comment) => comment.id === selected) ? selected : null })
      })
    },

    async add(pageId, markup) {
      const style = { color: useWorkspaceStore.getState().activeColor(), width: STROKE_WIDTH, author: '', date: pdfDate(new Date()) }
      await attempt(async () => {
        const added = await engine.request({ kind: 'addMarkup', pageId, markup, style })
        useDocumentStore.getState().touchPages(added.dirtyPages)
        await get().reload()
        set({ selected: added.id })
        if (markup.kind === 'note' || markup.kind === 'textBox') useWorkspaceStore.getState().showRight('comments')
      })
    },

    async remove(id) {
      const comment = commentOf(id)
      if (!comment) return
      await attempt(async () => {
        const change = await engine.request({ kind: 'deleteComment', pageId: comment.pageId, commentId: id })
        useDocumentStore.getState().touchPages(change.dirtyPages)
        await get().reload()
      })
    },

    async removeSelected() {
      const { selected } = get()
      if (selected) await get().remove(selected)
    },

    async setContents(id, contents) {
      const comment = commentOf(id)
      if (!comment || comment.contents === contents) return
      await attempt(async () => {
        const change = await engine.request({ kind: 'setCommentContents', commentId: id, contents, date: pdfDate(new Date()) })
        useDocumentStore.getState().touchPages(change.dirtyPages)
        await get().reload()
      })
    },

    select(id) {
      if (id !== get().selected) set({ selected: id })
    },

    reset() {
      sequence++
      set({ comments: [], selected: null, notice: null })
    },
  }
})

onDocumentReplaced((ready) => {
  useCommentsStore.getState().reset()
  if (ready) void useCommentsStore.getState().reload()
})

onDocumentChanged(() => void useCommentsStore.getState().reload())
