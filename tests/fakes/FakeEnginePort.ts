import { engine, type EngineClient, type EnginePort } from '@/services/EngineClient.ts'
import type { Request, Response, WorkerMessage } from '@/types/protocol.ts'

export interface SentMessage {
  message: WorkerMessage
  transfer: Transferable[]
}

export class FakeEnginePort implements EnginePort {
  onmessage: ((event: MessageEvent<Response>) => void) | null = null
  readonly sent: SentMessage[] = []

  static attach(client: EngineClient = engine): FakeEnginePort {
    const port = new FakeEnginePort()
    client.connect(port)
    return port
  }

  postMessage(message: WorkerMessage, transfer: Transferable[]) {
    this.sent.push({ message, transfer })
  }

  get requests(): Request[] {
    return this.sent.flatMap(({ message }) => (message.kind === 'cancel' ? [] : [message]))
  }

  get lastId(): number {
    return this.requests[this.requests.length - 1].id
  }

  answer(response: Response) {
    this.onmessage?.({ data: response } as MessageEvent<Response>)
  }

  async answerNext(result: unknown, revision = '') {
    await Promise.resolve()
    this.answer({ id: this.lastId, ok: true, result, revision })
  }
}
