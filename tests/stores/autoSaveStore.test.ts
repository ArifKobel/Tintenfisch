import { beforeEach, describe, expect, it, vi } from 'vitest'
import { documentLibrary } from '@/services/DocumentLibrary.ts'
import { engine } from '@/services/EngineClient.ts'
import { SAVE_DELAY, useAutoSaveStore } from '@/stores/autoSaveStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

let port: FakeEnginePort
let file: string

beforeEach(async () => {
  port = FakeEnginePort.attach()
  const opening = useDocumentStore.getState().open('report.pdf', new Uint8Array([1]))
  await port.answerNext({ status: 'opened', version: '1.7', byteLength: 1, objectCount: 3, pageCount: 1, repaired: false })
  await port.answerNext([{ id: '3-0', width: 612, height: 792, rotate: 0 }])
  await opening
  file = (await documentLibrary.add('report.pdf', new Uint8Array([1]))).id
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
  saver().track(file)
})

const saver = () => useAutoSaveStore.getState()

async function change(revision: string) {
  const rotating = engine.request({ kind: 'rotatePages', pageIds: ['3-0'], quarterTurns: 1 })
  await port.answerNext({ pages: [], dirtyPages: ['3-0'] }, revision)
  await rotating
}

async function answerSave(bytes: Uint8Array, revision: string) {
  await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('save'))
  await port.answerNext(bytes, revision)
  await saver().flush()
}

describe('autoSaveStore', () => {
  it('saves a changed document into the library once the edits pause', async () => {
    expect(saver().status).toBe('saved')
    await change('4.0')
    await change('5.0')
    expect(vi.getTimerCount()).toBe(1)
    expect(saver().status).toBe('pending')
    vi.advanceTimersByTime(SAVE_DELAY)
    await answerSave(new Uint8Array([9, 9]), '5.0')
    expect(saver().status).toBe('saved')
    expect((await documentLibrary.open(file))?.bytes).toEqual(new Uint8Array([9, 9]))
  })

  it('leaves untracked documents alone', async () => {
    saver().track(null)
    await change('4.0')
    expect(vi.getTimerCount()).toBe(0)
    expect(port.requests.at(-1)?.kind).toBe('rotatePages')
    expect(saver().status).toBe('idle')
  })

  it('reports a failed save', async () => {
    await documentLibrary.remove(file)
    await change('4.0')
    vi.advanceTimersByTime(SAVE_DELAY)
    await answerSave(new Uint8Array([9]), '4.0')
    expect(saver()).toMatchObject({ status: 'failed', error: 'the document is no longer stored' })
  })
})
