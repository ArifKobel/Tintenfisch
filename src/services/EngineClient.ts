import type { Command, Priority, Response, Results, WorkerMessage } from '@/types/protocol.ts'

export interface EnginePort {
  postMessage(message: WorkerMessage, transfer: Transferable[]): void
  onmessage: ((event: MessageEvent<Response>) => void) | null
}

export interface RequestOptions {
  priority?: Priority
  group?: string
  signal?: AbortSignal
}

export class CancelledError extends Error {
  constructor() {
    super('cancelled')
    this.name = 'CancelledError'
  }
}

interface PendingRequest {
  resolve: (result: unknown) => void
  reject: (error: Error) => void
}

export class EngineClient {
  readonly #port: EnginePort
  readonly #pending = new Map<number, PendingRequest>()
  readonly #changeListeners = new Set<() => void>()
  #nextId = 1
  #revision = ''

  constructor(port: EnginePort) {
    this.#port = port
    port.onmessage = (event) => this.#settle(event.data)
  }

  get revision(): string {
    return this.#revision
  }

  onChange(listener: () => void): () => void {
    this.#changeListeners.add(listener)
    return () => this.#changeListeners.delete(listener)
  }

  request<C extends Command>(command: C, options: RequestOptions = {}): Promise<Results[C['kind']]> {
    const { priority = 'interactive', group, signal } = options
    if (signal?.aborted) return Promise.reject(new CancelledError())
    const id = this.#nextId++
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve: resolve as (result: unknown) => void, reject })
      signal?.addEventListener('abort', () => this.#cancel(id), { once: true })
      this.#port.postMessage({ ...command, id, priority, group }, EngineClient.#transferablesOf(command))
    })
  }

  #cancel(id: number) {
    const pending = this.#pending.get(id)
    if (!pending) return
    this.#pending.delete(id)
    pending.reject(new CancelledError())
    this.#port.postMessage({ kind: 'cancel', ids: [id] }, [])
  }

  static #transferablesOf(command: Command): Transferable[] {
    return 'bytes' in command ? [command.bytes.buffer as ArrayBuffer] : []
  }

  #settle(response: Response) {
    if (response.ok && response.revision !== this.#revision) {
      this.#revision = response.revision
      this.#changeListeners.forEach((listener) => listener())
    }
    const pending = this.#pending.get(response.id)
    if (!pending) return
    this.#pending.delete(response.id)
    if (response.ok) pending.resolve(response.result)
    else pending.reject(response.cancelled ? new CancelledError() : new Error(response.error))
  }
}
