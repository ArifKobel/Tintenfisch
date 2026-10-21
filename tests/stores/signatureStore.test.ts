import { describe, expect, it } from 'vitest'
import type { SignatureMarkRequest } from '@/types/protocol.ts'
import { onDocumentChanged, useDocumentStore } from '@/stores/documentStore.ts'
import { useSignatureStore } from '@/stores/signatureStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const ink: SignatureMarkRequest = { kind: 'ink', strokes: [[[1, 2], [30, 4]]], penWidth: 2.5, color: [0, 0, 0.5] }
const rect = { x: 10, y: 20, width: 180, height: 60 }

function createStore() {
  const port = FakeEnginePort.attach()
  const store = useSignatureStore.getState()
  let changes = 0
  onDocumentChanged(() => changes++)
  return { port, store, changes: () => changes }
}

describe('signatureStore', () => {
  it('keeps the adopted signature and places it as often as asked', async () => {
    const { port, store, changes } = createStore()
    await store.place('3-0', rect)
    expect(port.requests).toHaveLength(0)
    store.openPad()
    store.adopt(ink)
    expect(useSignatureStore.getState()).toMatchObject({ padOpen: false, mark: ink })
    for (const pageId of ['3-0', '3-0']) {
      const placing = store.place(pageId, rect)
      expect(port.requests.at(-1)).toMatchObject({ kind: 'addSignature', pageId, rect, mark: ink })
      await port.answerNext({ fieldId: '70-0', dirtyPages: [pageId] })
      await placing
    }
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(2)
    expect(changes()).toBe(2)
  })

  it('shows why a signature was refused and forgets the notice once the pad opens again', async () => {
    const { port, store, changes } = createStore()
    store.adopt(ink)
    const placing = store.place('3-0', { ...rect, width: 0 })
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'a signature needs room on the page' })
    await placing
    expect(useSignatureStore.getState().notice).toBe('a signature needs room on the page')
    expect(changes()).toBe(0)
    store.openPad()
    expect(useSignatureStore.getState()).toMatchObject({ padOpen: true, notice: null, mark: ink })
  })
})
