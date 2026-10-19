import { IDBFactory } from 'fake-indexeddb'
import { describe, expect, it, vi } from 'vitest'
import { DocumentStore } from '@document/DocumentStore.ts'
import { EngineClient } from '@engine/EngineClient.ts'
import { AutoSave, SAVE_DELAY } from '@files/AutoSave.ts'
import { DocumentLibrary } from '@files/DocumentLibrary.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

async function createSaver() {
  const port = new FakeEnginePort()
  const client = new EngineClient(port)
  const documents = new DocumentStore(client)
  const library = new DocumentLibrary(new IDBFactory())
  const timers: { run: () => void; delay: number; cancelled: boolean }[] = []
  const saver = new AutoSave(client, documents, library, (run, delay) => {
    const timer = { run, delay, cancelled: false }
    timers.push(timer)
    return () => (timer.cancelled = true)
  })
  const opening = documents.open('report.pdf', new Uint8Array([1]))
  await port.answerNext({ status: 'opened', version: '1.7', byteLength: 1, objectCount: 3, pageCount: 1, repaired: false })
  await port.answerNext([{ id: '3-0', width: 612, height: 792, rotate: 0 }])
  await opening
  const file = await library.add('report.pdf', new Uint8Array([1]))
  saver.track(file.id)
  const change = async (revision: string) => {
    const rotating = client.request({ kind: 'rotatePages', pageIds: ['3-0'], quarterTurns: 1 })
    await port.answerNext({ pages: [], dirtyPages: ['3-0'] }, revision)
    await rotating
  }
  const runTimers = () => {
    for (const timer of timers.filter((candidate) => !candidate.cancelled)) {
      timer.cancelled = true
      timer.run()
    }
  }
  const answerSave = async (bytes: Uint8Array, revision: string) => {
    await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('save'))
    await port.answerNext(bytes, revision)
    await saver.flush()
  }
  return { port, library, saver, file, timers, change, runTimers, answerSave }
}

describe('AutoSave', () => {
  it('saves a changed document into the library once the edits pause', async () => {
    const { library, saver, file, timers, change, runTimers, answerSave } = await createSaver()
    expect(saver.getSnapshot().status).toBe('saved')
    await change('4.0')
    await change('5.0')
    expect(timers.filter((timer) => !timer.cancelled)).toHaveLength(1)
    expect(timers[0].delay).toBe(SAVE_DELAY)
    expect(saver.getSnapshot().status).toBe('pending')
    runTimers()
    await answerSave(new Uint8Array([9, 9]), '5.0')
    expect(saver.getSnapshot().status).toBe('saved')
    expect((await library.open(file.id))?.bytes).toEqual(new Uint8Array([9, 9]))
  })

  it('leaves untracked documents alone', async () => {
    const { port, saver, timers, change } = await createSaver()
    saver.track(null)
    await change('4.0')
    expect(timers).toHaveLength(0)
    expect(port.requests.at(-1)?.kind).toBe('rotatePages')
    expect(saver.getSnapshot().status).toBe('idle')
  })

  it('reports a failed save', async () => {
    const { library, saver, file, change, runTimers, answerSave } = await createSaver()
    await library.remove(file.id)
    await change('4.0')
    runTimers()
    await answerSave(new Uint8Array([9]), '4.0')
    expect(saver.getSnapshot()).toEqual({ status: 'failed', error: 'the document is no longer stored' })
  })
})
