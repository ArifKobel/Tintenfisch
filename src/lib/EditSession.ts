import type { ClusterRange, ClusterStyle, EditDraft, FontChoice, Point, Rgb, StyledRange, TextStyle } from '@/types/protocol.ts'

const graphemes = new Intl.Segmenter(undefined, { granularity: 'grapheme' })

export interface BoxChange {
  dx: number
  dy: number
  width: number | null
}

export interface TextLook {
  font: FontChoice | null
  size: number
  bold: boolean
  italic: boolean
  color: Rgb | null
}

const UNMOVED: BoxChange = { dx: 0, dy: 0, width: null }

const NEW_TEXT_LOOK: TextLook = { font: { kind: 'bundled', family: 'sans' }, size: 12, bold: false, italic: false, color: [0, 0, 0] }

interface SessionState {
  pageId: string
  line: number
  original: string
  clusters: readonly string[]
  looks: readonly TextLook[]
  styles: readonly StyledRange[]
  typing: TextStyle | null
  caret: number
  anchor: number | null
  box: BoxChange
  origin: Point | null
  newLook: TextLook
}

export class EditSession {
  readonly pageId: string
  readonly line: number
  readonly original: string
  readonly clusters: readonly string[]
  readonly looks: readonly TextLook[]
  readonly styles: readonly StyledRange[]
  readonly typing: TextStyle | null
  readonly caret: number
  readonly anchor: number | null
  readonly box: BoxChange
  readonly origin: Point | null
  readonly newLook: TextLook

  private constructor(state: SessionState) {
    this.pageId = state.pageId
    this.line = state.line
    this.original = state.original
    this.clusters = state.clusters
    this.looks = state.looks
    this.styles = state.styles
    this.typing = state.typing
    this.caret = EditSession.#clamp(state.caret, state.clusters.length)
    this.anchor = state.anchor === null ? null : EditSession.#clamp(state.anchor, state.clusters.length)
    this.box = state.box
    this.origin = state.origin
    this.newLook = state.newLook
  }

  static open(pageId: string, line: number, clusters: readonly string[], caret: number, styles: readonly ClusterStyle[] = []): EditSession {
    const looks = clusters.map((_, index) => EditSession.#lookOf(styles[index]))
    const newLook = looks[0] ?? NEW_TEXT_LOOK
    return new EditSession({ ...EditSession.#blank(pageId, null), line, original: clusters.join(''), clusters, looks, caret, newLook })
  }

  static openNew(pageId: string, origin: Point): EditSession {
    return new EditSession(EditSession.#blank(pageId, origin))
  }

  get text(): string {
    return this.clusters.join('')
  }

  get changed(): boolean {
    const { dx, dy, width } = this.box
    return this.text !== this.original || this.styles.length > 0 || dx !== 0 || dy !== 0 || width !== null
  }

  get selection(): ClusterRange | null {
    if (this.anchor === null || this.anchor === this.caret) return null
    return [Math.min(this.anchor, this.caret), Math.max(this.anchor, this.caret)]
  }

  get draft(): EditDraft {
    return { clusters: [...this.clusters], caret: this.caret, ...this.box, styles: [...this.styles], selection: this.selection }
  }

  get look(): TextLook {
    if (this.origin) return this.newLook
    const [start] = this.selection ?? [Math.max(this.caret - 1, 0)]
    const base = this.looks[start] ?? this.looks[0] ?? this.newLook
    return EditSession.#styled(base, this.typing ?? {})
  }

  insert(text: string): EditSession {
    const inserted = Array.from(graphemes.segment(text), (part) => part.segment)
    const [start, end] = this.selection ?? [this.caret, this.caret]
    const neighbor = this.looks[start - 1] ?? this.looks[end] ?? this.newLook
    const look = EditSession.#styled(neighbor, this.typing ?? {})
    const spliced = this.#spliced(start, end, inserted, inserted.map(() => look))
    const typing = this.typing
    const typedEnd = start + inserted.length
    if (!typing || inserted.length === 0 || spliced.styles.some((range) => range.start <= start && range.end >= typedEnd && EditSession.#carries(range, typing))) return spliced
    return new EditSession({ ...spliced.#state, styles: [...spliced.styles, { start, end: typedEnd, ...typing }] })
  }

  deleteBackward(): EditSession {
    const [start, end] = this.selection ?? [this.caret - 1, this.caret]
    if (start < 0) return this
    return this.#spliced(start, end, [], [])
  }

  deleteForward(): EditSession {
    const [start, end] = this.selection ?? [this.caret, this.caret + 1]
    if (end > this.clusters.length) return this
    return this.#spliced(start, end, [], [])
  }

  moveBy(delta: number): EditSession {
    return this.moveTo(this.caret + delta)
  }

  moveTo(caret: number): EditSession {
    return new EditSession({ ...this.#state, caret, anchor: null, typing: null })
  }

  moveToEnd(): EditSession {
    return this.moveTo(this.clusters.length)
  }

  extendBy(delta: number): EditSession {
    return this.extendTo(this.caret + delta)
  }

  extendTo(caret: number): EditSession {
    return new EditSession({ ...this.#state, caret, anchor: this.anchor ?? this.caret, typing: null })
  }

  selectWord(at: number): EditSession {
    if (this.clusters.length === 0) return this
    const blank = (index: number) => /^\s*$/u.test(this.clusters[index])
    const probe = Math.min(Math.max(at, 0), this.clusters.length - 1)
    const kind = blank(probe)
    let start = probe
    while (start > 0 && blank(start - 1) === kind) start--
    let end = probe + 1
    while (end < this.clusters.length && blank(end) === kind) end++
    return new EditSession({ ...this.#state, anchor: start, caret: end, typing: null })
  }

  selectAll(): EditSession {
    return new EditSession({ ...this.#state, anchor: 0, caret: this.clusters.length, typing: null })
  }

  formatted(style: TextStyle): EditSession {
    if (this.origin) return new EditSession({ ...this.#state, newLook: EditSession.#styled(this.newLook, style) })
    const selection = this.selection
    if (!selection) return new EditSession({ ...this.#state, typing: { ...this.typing, ...style } })
    const [start, end] = selection
    const looks = this.looks.map((look, index) => (index >= start && index < end ? EditSession.#styled(look, style) : look))
    return new EditSession({ ...this.#state, looks, styles: [...this.styles, { start, end, ...style }] })
  }

  movedTo(dx: number, dy: number): EditSession {
    return new EditSession({ ...this.#state, box: { ...this.box, dx, dy } })
  }

  resizedTo(width: number): EditSession {
    return new EditSession({ ...this.#state, box: { ...this.box, width } })
  }

  get #state(): SessionState {
    const { pageId, line, original, clusters, looks, styles, typing, caret, anchor, box, origin, newLook } = this
    return { pageId, line, original, clusters, looks, styles, typing, caret, anchor, box, origin, newLook }
  }

  #spliced(start: number, end: number, inserted: readonly string[], insertedLooks: readonly TextLook[]): EditSession {
    const added = inserted.length
    const removed = end - start
    const shiftStart = (at: number) => (at < start ? at : at >= end ? at - removed + added : start + added)
    const shiftEnd = (at: number) => (at < start ? at : at > end ? at - removed + added : start + added)
    const styles = this.styles
      .map((range) => ({ ...range, start: shiftStart(range.start), end: range.start < start && range.end === start ? range.end + added : shiftEnd(range.end) }))
      .filter((range) => range.start < range.end)
    return new EditSession({
      ...this.#state,
      clusters: [...this.clusters.slice(0, start), ...inserted, ...this.clusters.slice(end)],
      looks: [...this.looks.slice(0, start), ...insertedLooks, ...this.looks.slice(end)],
      styles,
      caret: start + added,
      anchor: null,
    })
  }

  static #blank(pageId: string, origin: Point | null): SessionState {
    return { pageId, line: -1, original: '', clusters: [], looks: [], styles: [], typing: null, caret: 0, anchor: null, box: UNMOVED, origin, newLook: NEW_TEXT_LOOK }
  }

  static #lookOf(style: ClusterStyle | undefined): TextLook {
    if (!style) return NEW_TEXT_LOOK
    const font: FontChoice | null = style.resource === null ? null : { kind: 'document', name: style.resource }
    return { font, size: style.size, bold: style.bold, italic: style.italic, color: style.color }
  }

  static #styled(look: TextLook, style: TextStyle): TextLook {
    return {
      font: style.font ?? look.font,
      size: style.size ?? look.size,
      bold: style.bold ?? look.bold,
      italic: style.italic ?? look.italic,
      color: style.color ?? look.color,
    }
  }

  static #carries(range: StyledRange, style: TextStyle): boolean {
    return (Object.keys(style) as (keyof TextStyle)[]).every((key) => JSON.stringify(range[key]) === JSON.stringify(style[key]))
  }

  static #clamp(value: number, length: number): number {
    return Math.min(Math.max(value, 0), length)
  }
}
