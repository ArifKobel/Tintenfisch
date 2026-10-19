import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { Permissions, ProtectionState } from '@engine/protocol.ts'

export interface ProtectionDialogState {
  dialogOpen: boolean
  current: ProtectionState | null
  notice: string | null
  pending: string | null
}

export type RandomBytes = (length: number) => Uint8Array

const ENTROPY = 68

type Listener = () => void

export class ProtectionStore {
  readonly #client: EngineClient
  readonly #random: RandomBytes
  readonly #listeners = new Set<Listener>()
  #state: ProtectionDialogState = { dialogOpen: false, current: null, notice: null, pending: null }

  constructor(client: EngineClient, random: RandomBytes = (length) => crypto.getRandomValues(new Uint8Array(length))) {
    this.#client = client
    this.#random = random
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): ProtectionDialogState => this.#state

  async openDialog() {
    await this.#attempt(async () => {
      const current = await this.#client.request({ kind: 'protection' })
      this.#update({ ...this.#state, dialogOpen: true, current, notice: null })
    })
  }

  closeDialog() {
    this.#update({ ...this.#state, dialogOpen: false, notice: null })
  }

  async grantOwnerRights(password: string) {
    await this.#attempt(async () => {
      const granted = await this.#client.request({ kind: 'grantOwnerRights', password })
      const current = await this.#client.request({ kind: 'protection' })
      this.#update({ ...this.#state, current, notice: granted ? null : 'That is not the permissions password' })
    })
  }

  async protect(userPassword: string, ownerPassword: string, permissions: Permissions) {
    await this.#attempt(async () => {
      await this.#client.request({ kind: 'protect', userPassword, ownerPassword, permissions, entropy: this.#random(ENTROPY) })
      this.#finish('The protection is written when you save the document')
    })
  }

  async unprotect() {
    await this.#attempt(async () => {
      await this.#client.request({ kind: 'unprotect' })
      this.#finish('The document is saved without protection from now on')
    })
  }

  reset() {
    this.#update({ dialogOpen: false, current: null, notice: null, pending: null })
  }

  #finish(pending: string) {
    this.#update({ ...this.#state, dialogOpen: false, notice: null, pending })
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: ProtectionDialogState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
