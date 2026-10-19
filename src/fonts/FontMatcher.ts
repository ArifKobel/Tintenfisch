import type { PageFont } from '@engine/protocol.ts'
import type { CatalogFamily, CatalogStyle } from '@fonts/FontCatalog.ts'

export interface FontMatch {
  family: CatalogFamily
  style: CatalogStyle
  exact: boolean
}

const WEIGHTS: Record<string, number> = {
  thin: 100,
  hairline: 100,
  extralight: 200,
  ultralight: 200,
  light: 300,
  regular: 400,
  normal: 400,
  book: 400,
  roman: 400,
  medium: 500,
  semibold: 600,
  demibold: 600,
  bold: 700,
  extrabold: 800,
  ultrabold: 800,
  black: 900,
  heavy: 900,
}

const SIMILAR: Record<PageFont['family'], string> = { sans: 'inter', serif: 'source-serif-4', mono: 'jetbrains-mono' }

const key = (text: string) => text.toLowerCase().replace(/[^a-z0-9]/g, '')

export class FontMatcher {
  readonly #families: CatalogFamily[]
  readonly #byName = new Map<string, CatalogFamily>()

  constructor(families: CatalogFamily[]) {
    this.#families = families
    for (const family of [...families].reverse()) this.#byName.set(key(family.family), family)
  }

  match(font: PageFont): FontMatch | null {
    const words = (font.font ?? '').split(/\s+/).filter(Boolean)
    for (let length = words.length; length > 0; length--) {
      const family = this.#byName.get(key(words.slice(0, length).join(' ')))
      if (family) return FontMatcher.#pick(family, FontMatcher.#weightOf(font, words.slice(length)), FontMatcher.#italicOf(font, words.slice(length)), true)
    }
    const similar = this.#families.find((family) => family.source === 'fontsource' && family.id === SIMILAR[font.family])
    return similar ? FontMatcher.#pick(similar, FontMatcher.#weightOf(font, words), FontMatcher.#italicOf(font, words), false) : null
  }

  static #weightOf(font: PageFont, styleWords: string[]): number {
    const named = styleWords.map((word) => WEIGHTS[key(word)]).find((weight) => weight !== undefined)
    return named ?? font.weight ?? (font.bold ? 700 : 400)
  }

  static #italicOf(font: PageFont, styleWords: string[]): boolean {
    return font.italic || styleWords.some((word) => /^(italic|oblique)$/i.test(word))
  }

  static styleOf(family: CatalogFamily, weight: number, italic: boolean): CatalogStyle | null {
    const slanted = family.styles.filter((style) => style.italic === italic)
    const candidates = slanted.length > 0 ? slanted : family.styles
    return candidates.reduce<CatalogStyle | null>((best, candidate) => (!best || Math.abs(candidate.weight - weight) < Math.abs(best.weight - weight) ? candidate : best), null)
  }

  static #pick(family: CatalogFamily, weight: number, italic: boolean, exact: boolean): FontMatch | null {
    const style = FontMatcher.styleOf(family, weight, italic)
    return style ? { family, style, exact } : null
  }
}
