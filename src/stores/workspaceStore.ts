import { create } from 'zustand'
import type { NewFieldKind, Rgb } from '@/types/protocol.ts'

export type Tool = 'select' | 'edit' | 'addText' | 'addImage' | 'highlight' | 'note' | 'textBox' | 'ink' | 'rectangle' | 'ellipse' | 'arrow' | 'stamp' | 'redactText' | 'redactArea' | 'form' | 'link' | 'sign'

export type LeftPanel = 'tools' | 'edit'
export type RightPanel = 'comments' | 'pages' | 'bookmarks'

export const EDITING_TOOLS: readonly Tool[] = ['edit', 'addText', 'addImage', 'link']

export const DRAWING_TOOLS: readonly Tool[] = ['note', 'textBox', 'ink', 'rectangle', 'ellipse', 'arrow', 'stamp', 'redactArea']

export const MARKUP_COLORS: readonly Rgb[] = [
  [0.86, 0.15, 0.15],
  [0.15, 0.39, 0.92],
  [0.09, 0.64, 0.29],
  [1, 0.83, 0.1],
]

export const STAMP_LABELS: readonly string[] = ['Approved', 'Draft', 'Confidential', 'Rejected']

interface WorkspaceStore {
  tool: Tool
  color: Rgb
  highlightColor: Rgb
  stampLabel: string
  fieldKind: NewFieldKind
  left: LeftPanel
  right: RightPanel | null
  sheet: boolean
  activeColor(): Rgb
  choose(tool: Tool): void
  setColor(color: Rgb): void
  setStampLabel(stampLabel: string): void
  setFieldKind(fieldKind: NewFieldKind): void
  showLeft(left: LeftPanel): void
  showRight(right: RightPanel | null): void
  toggleRight(right: RightPanel): void
  openSheet(): void
  closeSheet(): void
}

export const useWorkspaceStore = create<WorkspaceStore>()((set, get) => ({
  tool: 'select',
  color: MARKUP_COLORS[0],
  highlightColor: MARKUP_COLORS[3],
  stampLabel: STAMP_LABELS[0],
  fieldKind: 'text',
  left: 'tools',
  right: null,
  sheet: false,

  activeColor() {
    const { tool, color, highlightColor } = get()
    return tool === 'highlight' ? highlightColor : color
  },

  choose(tool) {
    const { left } = get()
    set({ tool, left: EDITING_TOOLS.includes(tool) ? 'edit' : left === 'edit' ? 'tools' : left })
  },

  setColor(color) {
    set(get().tool === 'highlight' ? { highlightColor: color } : { color })
  },

  setStampLabel(stampLabel) {
    set({ stampLabel })
  },

  setFieldKind(fieldKind) {
    set({ fieldKind })
  },

  showLeft(left) {
    const { tool } = get()
    const editing = EDITING_TOOLS.includes(tool)
    if (left === 'edit' && !editing) set({ left, tool: 'edit' })
    else if (left !== 'edit' && editing) set({ left, tool: 'select' })
    else if (left !== get().left) set({ left })
  },

  showRight(right) {
    const { sheet } = get()
    if (right !== get().right) set({ right, sheet: right === null && sheet })
  },

  toggleRight(right) {
    get().showRight(get().right === right ? null : right)
  },

  openSheet() {
    if (!get().sheet) set({ sheet: true })
  },

  closeSheet() {
    if (get().sheet) set({ sheet: false })
  },
}))

export function onToolChosen(listener: (tool: Tool, previous: Tool) => void): () => void {
  return useWorkspaceStore.subscribe(({ tool }, previous) => {
    if (tool !== previous.tool) listener(tool, previous.tool)
  })
}
