export type FontDownloads = 'unknown' | 'allowed' | 'refused'

export interface FontConsentState {
  downloads: FontDownloads
  asking: boolean
}

export type ConsentStorage = Pick<Storage, 'getItem' | 'setItem'>

const KEY = 'tintenfisch.fontDownloads'

type Listener = () => void

export class FontConsent {
  readonly #storage: ConsentStorage | null
  readonly #listeners = new Set<Listener>()
  #state: FontConsentState

  constructor(storage: ConsentStorage | null) {
    this.#storage = storage
    this.#state = { downloads: this.#read(), asking: false }
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): FontConsentState => this.#state

  askOnce() {
    if (this.#state.downloads === 'unknown') this.review()
  }

  review() {
    if (!this.#state.asking) this.#update({ ...this.#state, asking: true })
  }

  answer(allowed: boolean) {
    const downloads = allowed ? 'allowed' : 'refused'
    this.#remember(downloads)
    this.#update({ downloads, asking: false })
  }

  #remember(downloads: FontDownloads) {
    try {
      this.#storage?.setItem(KEY, downloads)
    } catch (error) {
      if (!(error instanceof DOMException)) throw error
    }
  }

  #read(): FontDownloads {
    try {
      const stored = this.#storage?.getItem(KEY)
      return stored === 'allowed' || stored === 'refused' ? stored : 'unknown'
    } catch {
      return 'unknown'
    }
  }

  #update(state: FontConsentState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
