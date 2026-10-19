import { describe, expect, it } from 'vitest'
import type { Request, Response } from '@engine/protocol.ts'
import { WorkerRuntime } from '@engine/WorkerRuntime.ts'

function runtime() {
  const handled: number[] = []
  const posted: Response[] = []
  const waiting: Array<() => void> = []
  const handler = {
    handle: (request: Request) => {
      handled.push(request.id)
      return { response: { id: request.id, ok: true as const, result: null, revision: '' }, transfer: [] }
    },
  }
  const worker = new WorkerRuntime(Promise.resolve(handler), (response) => posted.push(response), (resume) => waiting.push(resume))
  const step = async () => {
    waiting.shift()?.()
    await new Promise((resolve) => setTimeout(resolve, 0))
  }
  return { worker, handled, posted, step }
}

const pages = (id: number, priority: Request['priority'], group?: string): Request => ({ id, priority, group, kind: 'pages' })

describe('WorkerRuntime', () => {
  it('runs one request per turn so newer messages are seen in between', async () => {
    const { worker, handled, step } = runtime()
    worker.receive(pages(1, 'prefetch'))
    worker.receive(pages(2, 'prefetch'))
    await step()
    worker.receive(pages(3, 'interactive'))
    await step()
    await step()
    expect(handled).toEqual([1, 3, 2])
  })

  it('answers cancelled and superseded requests without running them', async () => {
    const { worker, handled, posted, step } = runtime()
    worker.receive(pages(1, 'visible', 'tiles'))
    worker.receive(pages(2, 'visible', 'tiles'))
    worker.receive(pages(3, 'visible'))
    worker.receive({ kind: 'cancel', ids: [3] })
    await step()
    await step()
    expect(handled).toEqual([2])
    expect(posted.filter((response) => !response.ok).map((response) => response.id)).toEqual([1, 3])
  })
})
