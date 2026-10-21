import { CommentsStore } from '@/stores/CommentsStore.ts'
import type { DocumentStore } from '@/stores/DocumentStore.ts'
import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { SignatureInfo, SigningRequest } from '@/types/protocol.ts'

export interface DigitalSignatureState {
  dialogOpen: boolean
  signing: boolean
  notice: string | null
  reports: SignatureInfo[]
}

type Listener = () => void

const INITIAL: DigitalSignatureState = { dialogOpen: false, signing: false, notice: null, reports: [] }

export class DigitalSignatureStore {
  readonly #client: EngineClient
  readonly #documents: DocumentStore
  readonly #now: () => Date
  readonly #listeners = new Set<Listener>()
  #state: DigitalSignatureState = INITIAL

  constructor(client: EngineClient, documents: DocumentStore, now: () => Date = () => new Date()) {
    this.#client = client
    this.#documents = documents
    this.#now = now
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): DigitalSignatureState => this.#state

  static signedName(name: string): string {
    const base = name.replace(/\.pdf$/i, '')
    return `${base.replace(/-signed$/, '')}-signed.pdf`
  }

  async reload() {
    try {
      const reports = await this.#client.request({ kind: 'signatures' }, { group: 'signatures' })
      this.#update({ ...this.#state, reports })
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  openDialog() {
    this.#update({ ...this.#state, dialogOpen: true, notice: null })
  }

  closeDialog() {
    if (!this.#state.signing) this.#update({ ...this.#state, dialogOpen: false })
  }

  async sign(request: SigningRequest): Promise<{ name: string; bytes: Uint8Array } | null> {
    const document = this.#documents.getSnapshot()
    if (document.phase !== 'ready' || this.#state.signing) return null
    this.#update({ ...this.#state, signing: true, notice: null })
    try {
      const bytes = await this.#client.request({ kind: 'signField', ...request, date: CommentsStore.pdfDate(this.#now()) })
      const name = DigitalSignatureStore.signedName(document.name)
      this.#update({ ...this.#state, signing: false, dialogOpen: false })
      await this.#documents.open(name, bytes.slice())
      return { name, bytes }
    } catch (error) {
      const notice = error instanceof CancelledError ? null : (error as Error).message
      this.#update({ ...this.#state, signing: false, notice })
      return null
    }
  }

  reset() {
    this.#update(INITIAL)
  }

  #update(state: DigitalSignatureState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
