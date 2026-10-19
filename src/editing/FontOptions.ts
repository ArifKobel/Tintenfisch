import type { PageFont } from '@engine/protocol.ts'

export interface FontOption {
  name: string
  label: string
}

const UNNAMED = 'Unnamed font'

export class FontOptions {
  readonly options: FontOption[]
  readonly #shownAs = new Map<string, string>()

  constructor(fonts: readonly PageFont[]) {
    const unnamed = fonts.filter((font) => font.font === null).length
    const byLabel = new Map<string, FontOption>()
    let ordinal = 0
    for (const font of fonts) {
      const label = font.font ?? (unnamed > 1 ? `${UNNAMED} ${++ordinal}` : UNNAMED)
      const option = byLabel.get(label) ?? { name: font.name, label }
      byLabel.set(label, option)
      this.#shownAs.set(font.name, option.name)
    }
    this.options = [...byLabel.values()]
  }

  shownAs(name: string): string {
    return this.#shownAs.get(name) ?? name
  }
}
