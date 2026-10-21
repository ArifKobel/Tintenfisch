import type { EditingStore } from '@/stores/EditingStore.ts'
import type { EngineClient } from '@/services/EngineClient.ts'
import type { FontChoice, PageFont } from '@/types/protocol.ts'
import type { CatalogFamily, CatalogStyle, FontCatalog } from '@/services/FontCatalog.ts'
import type { FontConsent } from '@/stores/FontConsent.ts'
import { FontMatcher } from '@/lib/FontMatcher.ts'

export type Download = (url: string) => Promise<Uint8Array>

export class FontReplacements {
  readonly #client: EngineClient
  readonly #editing: EditingStore
  readonly #consent: FontConsent
  readonly #catalog: FontCatalog
  readonly #download: Download
  readonly #tried = new Set<string>()
  readonly #loaded = new Set<string>()
  #matcher: Promise<FontMatcher> | null = null

  constructor(client: EngineClient, editing: EditingStore, consent: FontConsent, catalog: FontCatalog, download: Download) {
    this.#client = client
    this.#editing = editing
    this.#consent = consent
    this.#catalog = catalog
    this.#download = download
    let fonts = editing.getSnapshot().fonts
    editing.subscribe(() => {
      if (editing.getSnapshot().fonts === fonts) return
      fonts = editing.getSnapshot().fonts
      void this.supply()
    })
    consent.subscribe(() => void this.supply())
  }

  async supply() {
    const wanted = this.#editing.getSnapshot().fonts.filter((font) => font.embedded && !this.#tried.has(font.baseFont))
    if (wanted.length === 0) return
    const { downloads } = this.#consent.getSnapshot()
    if (downloads === 'unknown') this.#consent.askOnce()
    if (downloads !== 'allowed') return
    wanted.forEach((font) => this.#tried.add(font.baseFont))
    this.#matcher ??= this.#catalog.families().then((families) => new FontMatcher(families))
    const matcher = await this.#matcher
    const supplied = await Promise.all(wanted.map((font) => this.#supplyOne(matcher, font)))
    if (supplied.some(Boolean)) await this.#editing.refresh()
  }

  async catalogFont(family: CatalogFamily, weight: number, italic: boolean): Promise<FontChoice | null> {
    const style = FontMatcher.styleOf(family, weight, italic)
    if (!style || !(await this.#consent.request())) return null
    const key = `${family.source}:${family.id}:${style.weight}:${style.italic ? 'italic' : 'normal'}`
    if (!this.#loaded.has(key)) {
      await this.#register(key, style)
      this.#loaded.add(key)
    }
    return { kind: 'catalog', key, source: family.source, id: family.id, family: family.family, weight: style.weight, italic: style.italic }
  }

  async #supplyOne(matcher: FontMatcher, font: PageFont): Promise<boolean> {
    const match = matcher.match(font)
    if (!match) return false
    try {
      await this.#register(font.baseFont, match.style)
      return true
    } catch {
      return false
    }
  }

  async #register(key: string, style: CatalogStyle) {
    for (const bytes of await Promise.all(style.urls.map((url) => this.#download(url)))) {
      await this.#client.request({ kind: 'registerFontReplacement', key, bytes })
    }
  }
}
