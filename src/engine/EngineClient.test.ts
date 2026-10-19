import { describe, expect, it } from 'vitest'
import { CancelledError, EngineClient } from '@engine/EngineClient.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

const opened = { version: '1.7', byteLength: 1, objectCount: 5, pageCount: 1, repaired: true }

describe('EngineClient', () => {
  it('matches responses to requests by id, in any order', async () => {
    const port = new FakeEnginePort()
    const client = new EngineClient(port)
    const first = client.request({ kind: 'open', bytes: new Uint8Array(1) })
    const second = client.request({ kind: 'pages' })
    const [a, b] = port.requests.map((request) => request.id)
    port.answer({ id: b, ok: true, result: [], revision: 0 })
    port.answer({ id: a, ok: true, result: opened, revision: 0 })
    expect(await first).toEqual(opened)
    expect(await second).toEqual([])
  })

  it('rejects with the engine error message', async () => {
    const port = new FakeEnginePort()
    const result = new EngineClient(port).request({ kind: 'open', bytes: new Uint8Array(0) })
    port.answer({ id: port.lastId, ok: false, error: 'not a PDF file' })
    await expect(result).rejects.toThrow('not a PDF file')
  })

  it('transfers the document bytes and passes priority and group along', () => {
    const port = new FakeEnginePort()
    const bytes = new Uint8Array(8)
    void new EngineClient(port).request({ kind: 'open', bytes }, { priority: 'visible', group: 'documents' })
    expect(port.sent[0].transfer).toEqual([bytes.buffer])
    expect(port.requests[0]).toMatchObject({ priority: 'visible', group: 'documents' })
  })

  it('cancels through an abort signal and tells the worker', async () => {
    const port = new FakeEnginePort()
    const controller = new AbortController()
    const result = new EngineClient(port).request({ kind: 'pages' }, { signal: controller.signal })
    controller.abort()
    await expect(result).rejects.toBeInstanceOf(CancelledError)
    expect(port.sent[1].message).toEqual({ kind: 'cancel', ids: [port.lastId] })
  })

  it('turns a cancelled response into a CancelledError', async () => {
    const port = new FakeEnginePort()
    const result = new EngineClient(port).request({ kind: 'pages' })
    port.answer({ id: port.lastId, ok: false, error: 'cancelled', cancelled: true })
    await expect(result).rejects.toBeInstanceOf(CancelledError)
  })

  it('ignores responses nobody waits for', () => {
    const port = new FakeEnginePort()
    void new EngineClient(port)
    expect(() => port.answer({ id: 99, ok: true, result: null, revision: 0 })).not.toThrow()
  })

  it('tells listeners when a reply carries a new document revision', async () => {
    const port = new FakeEnginePort()
    const client = new EngineClient(port)
    let changes = 0
    client.onChange(() => changes++)
    const pages = client.request({ kind: 'pages' })
    await port.answerNext([], 0)
    await pages
    const rotated = client.request({ kind: 'rotatePages', pageIds: ['3-0'], quarterTurns: 1 })
    await port.answerNext({ pages: [], dirtyPages: ['3-0'] }, 7)
    await rotated
    expect(changes).toBe(1)
    expect(client.revision).toBe(7)
  })
})
