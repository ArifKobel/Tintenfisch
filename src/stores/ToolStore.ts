import type { NewFieldKind, Rgb } from '@/types/protocol.ts'

export type Tool = 'select' | 'edit' | 'addText' | 'addImage' | 'highlight' | 'note' | 'textBox' | 'ink' | 'rectangle' | 'ellipse' | 'arrow' | 'stamp' | 'redactText' | 'redactArea' | 'form' | 'link' | 'sign'

export const EDITING_TOOLS: readonly Tool[] = ['edit', 'addText', 'addImage', 'link']

export const DRAWING_TOOLS: readonly Tool[] = ['note', 'textBox', 'ink', 'rectangle', 'ellipse', 'arrow', 'stamp', 'redactArea']

export const MARKUP_COLORS: readonly Rgb[] = [
  [0.86, 0.15, 0.15],
  [0.15, 0.39, 0.92],
  [0.09, 0.64, 0.29],
  [1, 0.83, 0.1],
]

export const STAMP_LABELS: readonly string[] = ['Approved', 'Draft', 'Confidential', 'Rejected']

export interface ToolState {
  tool: Tool
  color: Rgb
  highlightColor: Rgb
  stampLabel: string
  fieldKind: NewFieldKind
}

type Listener = () => void

export class ToolStore {
  readonly #listeners = new Set<Listener>()
  #state: ToolState = { tool: 'select', color: MARKUP_COLORS[0], highlightColor: MARKUP_COLORS[3], stampLabel: STAMP_LABELS[0], fieldKind: 'text' }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): ToolState => this.#state

  get activeColor(): Rgb {
    return this.#state.tool === 'highlight' ? this.#state.highlightColor : this.#state.color
  }

  choose(tool: Tool) {
    this.#update({ ...this.#state, tool })
  }

  setColor(color: Rgb) {
    this.#update(this.#state.tool === 'highlight' ? { ...this.#state, highlightColor: color } : { ...this.#state, color })
  }

  setStampLabel(stampLabel: string) {
    this.#update({ ...this.#state, stampLabel })
  }

  setFieldKind(fieldKind: NewFieldKind) {
    this.#update({ ...this.#state, fieldKind })
  }

  #update(state: ToolState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
