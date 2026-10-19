import { writeFile } from 'node:fs/promises'

interface FontsourceFamily {
  id: string
  family: string
  category: string
  weights: number[]
  styles: string[]
  subsets: string[]
}

interface FontshareStyle {
  file: string
  weight: { number: number }
  is_italic: boolean
  is_variable: boolean
}

interface FontshareFamily {
  name: string
  slug: string
  category: string
  styles: FontshareStyle[]
}

const FONTSHARE_FILES = '//cdn.fontshare.com/wf/'
const TARGET = new URL('../public/fonts/catalog.json', import.meta.url)

async function json<T>(url: string): Promise<T> {
  const response = await fetch(url, { headers: { 'user-agent': 'Tintenfisch font catalog snapshot' } })
  if (!response.ok) throw new Error(`${url} answered ${response.status}`)
  return (await response.json()) as T
}

async function fontsource() {
  const families = await json<FontsourceFamily[]>('https://api.fontsource.org/v1/fonts')
  return families
    .filter((family) => family.subsets.includes('latin'))
    .map((family) => [family.id, family.family, family.category, family.weights, family.styles.includes('italic'), family.subsets.includes('latin-ext')])
}

async function fontshare() {
  const families: FontshareFamily[] = []
  for (let offset = 0; ; offset += 100) {
    const page = await json<{ fonts: FontshareFamily[]; has_more: boolean }>(`https://api.fontshare.com/v2/fonts?limit=100&offset=${offset}`)
    families.push(...page.fonts)
    if (!page.has_more) break
  }
  return families.map((family) => [
    family.slug,
    family.name,
    family.category.toLowerCase(),
    family.styles
      .filter((style) => !style.is_variable && style.file.startsWith(FONTSHARE_FILES))
      .map((style) => [style.weight.number, style.is_italic, style.file.slice(FONTSHARE_FILES.length)]),
  ])
}

const catalog = { fontsource: await fontsource(), fontshare: await fontshare() }
await writeFile(TARGET, JSON.stringify(catalog))
console.log(`${catalog.fontsource.length} Fontsource and ${catalog.fontshare.length} Fontshare families written to ${TARGET.pathname}`)
