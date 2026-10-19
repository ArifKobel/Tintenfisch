import type { PageFont } from '@engine/protocol.ts'

const UNNAMED = 'Unnamed font'

export class FontLabels {
  static of(fonts: readonly PageFont[]): Map<string, string> {
    const named = fonts.map((font) => [font.name, font.font ?? UNNAMED] as const)
    const total = new Map<string, number>()
    for (const [, label] of named) total.set(label, (total.get(label) ?? 0) + 1)
    const seen = new Map<string, number>()
    return new Map(
      named.map(([name, label]) => {
        if (total.get(label) === 1) return [name, label]
        const ordinal = (seen.get(label) ?? 0) + 1
        seen.set(label, ordinal)
        return [name, `${label} ${ordinal}`]
      }),
    )
  }
}
