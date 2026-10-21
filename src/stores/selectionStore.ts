import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import type { SelectionUnit, TextPoint, TextSelection } from '@/types/protocol.ts'

export type Selection = ({ pageId: string; unit: SelectionUnit } & TextSelection) | null

interface SelectionStore {
  selection: Selection
  begin(pageId: string, x: number, y: number, unit: SelectionUnit): Promise<void>
  extend(pageId: string, x: number, y: number): Promise<void>
  clear(): void
}

async function unlessCancelled(work: () => Promise<void>) {
  try {
    await work()
  } catch (error) {
    if (!(error instanceof CancelledError)) throw error
  }
}

export const useSelectionStore = create<SelectionStore>()((set, get) => {
  let anchor: { pageId: string; point: TextPoint; unit: SelectionUnit } | null = null
  let sequence = 0

  const select = async (pageId: string, from: TextPoint, to: TextPoint, unit: SelectionUnit, current: number) => {
    const selection = await engine.request({ kind: 'selectText', pageId, anchor: from, focus: to, unit }, { group: 'selection' })
    if (current === sequence) set({ selection: { pageId, unit, ...selection } })
  }

  return {
    selection: null,

    begin: (pageId, x, y, unit) =>
      unlessCancelled(async () => {
        const current = ++sequence
        const point = await engine.request({ kind: 'hitText', pageId, x, y })
        if (current !== sequence) return
        if (!point) {
          get().clear()
          return
        }
        anchor = { pageId, point, unit }
        await select(pageId, point, point, unit, current)
      }),

    extend: (pageId, x, y) =>
      unlessCancelled(async () => {
        const from = anchor
        if (!from || from.pageId !== pageId) return
        const current = ++sequence
        const point = await engine.request({ kind: 'hitText', pageId, x, y }, { group: 'selection-hit' })
        if (current !== sequence || !point) return
        await select(pageId, from.point, point, from.unit, current)
      }),

    clear() {
      sequence++
      anchor = null
      set({ selection: null })
    },
  }
})
