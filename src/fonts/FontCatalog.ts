export type FontCategory = 'sans' | 'serif' | 'mono' | 'display' | 'handwriting' | 'other'

export interface CatalogStyle {
  weight: number
  italic: boolean
  urls: string[]
}

export interface CatalogFamily {
  source: 'fontsource' | 'fontshare'
  id: string
  family: string
  category: FontCategory
  styles: CatalogStyle[]
}

type FontsourceEntry = [id: string, family: string, category: string, weights: number[], italic: boolean, latinExtended: boolean]
type FontshareEntry = [slug: string, name: string, category: string, styles: [weight: number, italic: boolean, path: string][]]

export interface CatalogSnapshot {
  fontsource: FontsourceEntry[]
  fontshare: FontshareEntry[]
}

const CATEGORIES: Record<string, FontCategory> = {
  'sans-serif': 'sans',
  sans: 'sans',
  serif: 'serif',
  slab: 'serif',
  monospace: 'mono',
  display: 'display',
  handwriting: 'handwriting',
  handwritten: 'handwriting',
}

const fontsourceUrl = (id: string, subset: string, weight: number, style: string) =>
  `https://cdn.jsdelivr.net/fontsource/fonts/${id}@latest/${subset}-${weight}-${style}.ttf`

export class FontCatalog {
  readonly #load: () => Promise<CatalogSnapshot>
  #families: Promise<CatalogFamily[]> | null = null

  constructor(load: () => Promise<CatalogSnapshot>) {
    this.#load = load
  }

  families(): Promise<CatalogFamily[]> {
    this.#families ??= this.#load().then(FontCatalog.parse)
    return this.#families
  }

  static parse(snapshot: CatalogSnapshot): CatalogFamily[] {
    const fontsource = snapshot.fontsource.map(([id, family, category, weights, italic, latinExtended]): CatalogFamily => {
      const subsets = latinExtended ? ['latin', 'latin-ext'] : ['latin']
      const styles = (italic ? ['normal', 'italic'] : ['normal']).flatMap((style) =>
        weights.map((weight) => ({ weight, italic: style === 'italic', urls: subsets.map((subset) => fontsourceUrl(id, subset, weight, style)) })),
      )
      return { source: 'fontsource', id, family, category: CATEGORIES[category] ?? 'other', styles }
    })
    const fontshare = snapshot.fontshare.map(([slug, family, category, styles]): CatalogFamily => ({
      source: 'fontshare',
      id: slug,
      family,
      category: CATEGORIES[category] ?? 'other',
      styles: styles.map(([weight, italic, path]) => ({ weight, italic, urls: [`https://cdn.fontshare.com/wf/${path}.ttf`] })),
    }))
    return [...fontsource, ...fontshare]
  }
}
