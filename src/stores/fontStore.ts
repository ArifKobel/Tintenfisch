import { create } from 'zustand'
import { FontMatcher } from '@/lib/FontMatcher.ts'
import { engine } from '@/services/EngineClient.ts'
import { catalogFamilies, downloadFont, type CatalogFamily, type CatalogStyle } from '@/services/fontCatalog.ts'
import { useEditingStore } from '@/stores/editingStore.ts'
import type { FontChoice, PageFont } from '@/types/protocol.ts'

export type FontDownloads = 'unknown' | 'allowed' | 'refused'

const KEY = 'tintenfisch.fontDownloads'

interface FontStore {
  downloads: FontDownloads
  asking: boolean
  request(): Promise<boolean>
  askOnce(): void
  review(): void
  answer(allowed: boolean): void
  supply(): Promise<void>
  catalogFont(family: CatalogFamily, weight: number, italic: boolean): Promise<FontChoice | null>
}

function remembered(): FontDownloads {
  try {
    const stored = localStorage.getItem(KEY)
    return stored === 'allowed' || stored === 'refused' ? stored : 'unknown'
  } catch {
    return 'unknown'
  }
}

function remember(downloads: FontDownloads) {
  try {
    localStorage.setItem(KEY, downloads)
  } catch (error) {
    if (!(error instanceof DOMException) && !(error instanceof ReferenceError)) throw error
  }
}

async function register(key: string, style: CatalogStyle) {
  for (const bytes of await Promise.all(style.urls.map((url) => downloadFont(url)))) {
    await engine.request({ kind: 'registerFontReplacement', key, bytes })
  }
}

export const useFontStore = create<FontStore>()((set, get) => {
  const tried = new Set<string>()
  const loaded = new Set<string>()
  let matcher: Promise<FontMatcher> | null = null

  const supplyOne = async (fonts: FontMatcher, font: PageFont): Promise<boolean> => {
    const match = fonts.match(font)
    if (!match) return false
    try {
      await register(font.baseFont, match.style)
      return true
    } catch {
      return false
    }
  }

  return {
    downloads: remembered(),
    asking: false,

    request() {
      if (get().downloads === 'allowed') return Promise.resolve(true)
      get().review()
      return new Promise((resolve) => {
        const stop = useFontStore.subscribe(({ asking, downloads }) => {
          if (asking) return
          stop()
          resolve(downloads === 'allowed')
        })
      })
    },

    askOnce() {
      if (get().downloads === 'unknown') get().review()
    },

    review() {
      if (!get().asking) set({ asking: true })
    },

    answer(allowed) {
      const downloads = allowed ? 'allowed' : 'refused'
      remember(downloads)
      set({ downloads, asking: false })
    },

    async supply() {
      const wanted = useEditingStore.getState().fonts.filter((font) => font.embedded && !tried.has(font.baseFont))
      if (wanted.length === 0) return
      const { downloads } = get()
      if (downloads === 'unknown') get().askOnce()
      if (downloads !== 'allowed') return
      wanted.forEach((font) => tried.add(font.baseFont))
      matcher ??= catalogFamilies().then((families) => new FontMatcher(families))
      const fonts = await matcher
      const supplied = await Promise.all(wanted.map((font) => supplyOne(fonts, font)))
      if (supplied.some(Boolean)) await useEditingStore.getState().refresh()
    },

    async catalogFont(family, weight, italic) {
      const style = FontMatcher.styleOf(family, weight, italic)
      if (!style || !(await get().request())) return null
      const key = `${family.source}:${family.id}:${style.weight}:${style.italic ? 'italic' : 'normal'}`
      if (!loaded.has(key)) {
        await register(key, style)
        loaded.add(key)
      }
      return { kind: 'catalog', key, source: family.source, id: family.id, family: family.family, weight: style.weight, italic: style.italic }
    },
  }
})

useEditingStore.subscribe(({ fonts }, previous) => {
  if (fonts !== previous.fonts) void useFontStore.getState().supply()
})

useFontStore.subscribe(({ downloads, asking }, previous) => {
  if (downloads !== previous.downloads || asking !== previous.asking) void useFontStore.getState().supply()
})
