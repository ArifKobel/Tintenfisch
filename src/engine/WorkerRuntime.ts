import type { HostReply } from '@engine/EngineHost.ts'
import { RequestScheduler } from '@engine/RequestScheduler.ts'
import type { Request, Response, WorkerMessage } from '@engine/protocol.ts'

export interface RequestHandler {
  handle(request: Request): HostReply
}

export type Post = (response: Response, transfer: Transferable[]) => void

export class WorkerRuntime {
  readonly #handler: Promise<RequestHandler>
  readonly #post: Post
  readonly #yieldToMessages: (resume: () => void) => void
  readonly #scheduler = new RequestScheduler<Request>()
  #draining = false

  constructor(handler: Promise<RequestHandler>, post: Post, yieldToMessages: (resume: () => void) => void) {
    this.#handler = handler
    this.#post = post
    this.#yieldToMessages = yieldToMessages
  }

  receive(message: WorkerMessage) {
    const dropped = message.kind === 'cancel' ? this.#scheduler.cancel(message.ids) : this.#scheduler.enqueue(message)
    for (const request of dropped) {
      this.#post({ id: request.id, ok: false, error: 'cancelled', cancelled: true }, [])
    }
    this.#scheduleDrain()
  }

  #scheduleDrain() {
    if (this.#draining || this.#scheduler.size === 0) return
    this.#draining = true
    this.#yieldToMessages(() => void this.#drainOne())
  }

  async #drainOne() {
    const handler = await this.#handler
    const request = this.#scheduler.next()
    if (request) {
      const { response, transfer } = handler.handle(request)
      this.#post(response, transfer)
    }
    this.#draining = false
    this.#scheduleDrain()
  }
}
