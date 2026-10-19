import type { PageFont } from '@engine/protocol.ts'

export class FontLabels {
  static of(fonts: readonly PageFont[]): Map<string, string> {
    const named = fonts.map((font) => [font.name, font.font || font.name] as const)
    const counts = new Map<string, number>()
    for (const [, label] of named) counts.set(label, (counts.get(label) ?? 0) + 1)
    return new Map(named.map(([name, label]) => [name, (counts.get(label) ?? 0) > 1 ? `${label} (${name})` : label]))
  }
}
