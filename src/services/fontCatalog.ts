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

export function parseCatalog(snapshot: CatalogSnapshot): CatalogFamily[] {
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

async function fetched(url: string): Promise<Response> {
  const response = await fetch(url)
  if (!response.ok) throw new Error(`${url} answered ${response.status}`)
  return response
}

let families: Promise<CatalogFamily[]> | null = null

export function catalogFamilies(): Promise<CatalogFamily[]> {
  families ??= fetched('/fonts/catalog.json').then(async (response) => parseCatalog((await response.json()) as CatalogSnapshot))
  return families
}

export async function downloadFont(url: string): Promise<Uint8Array> {
  return new Uint8Array(await (await fetched(url)).arrayBuffer())
}
