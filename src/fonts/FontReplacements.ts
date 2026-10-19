import type { EditingStore } from '@editing/EditingStore.ts'
import type { EngineClient } from '@engine/EngineClient.ts'
import type { PageFont } from '@engine/protocol.ts'
import type { FontCatalog } from '@fonts/FontCatalog.ts'
import type { FontConsent } from '@fonts/FontConsent.ts'
import { FontMatcher } from '@fonts/FontMatcher.ts'

export type Download = (url: string) => Promise<Uint8Array>

export class FontReplacements {
  readonly #client: EngineClient
  readonly #editing: EditingStore
  readonly #consent: FontConsent
  readonly #catalog: FontCatalog
  readonly #download: Download
  readonly #tried = new Set<string>()
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

  async #supplyOne(matcher: FontMatcher, font: PageFont): Promise<boolean> {
    const match = matcher.match(font)
    if (!match) return false
    try {
      for (const bytes of await Promise.all(match.style.urls.map((url) => this.#download(url)))) {
        await this.#client.request({ kind: 'registerFontReplacement', baseFont: font.baseFont, bytes })
      }
      return true
    } catch {
      return false
    }
  }
}
