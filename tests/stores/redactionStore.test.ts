import { describe, expect, it } from 'vitest'
import { onDocumentChanged, useDocumentStore } from '@/stores/documentStore.ts'
import { useRedactionStore } from '@/stores/redactionStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

function createStore() {
  const port = FakeEnginePort.attach()
  const store = useRedactionStore.getState()
  let changes = 0
  onDocumentChanged(() => changes++)
  return { port, store, changes: () => changes }
}

describe('redactionStore', () => {
  it('applies only after confirming, then redraws the changed pages and announces the change', async () => {
    const { port, store, changes } = createStore()
    await store.apply()
    expect(port.requests).toHaveLength(0)
    store.requestApply()
    expect(useRedactionStore.getState().confirming).toBe(true)
    const applying = store.apply()
    expect(useRedactionStore.getState()).toMatchObject({ confirming: false, applying: true })
    expect(port.requests[0]).toMatchObject({ kind: 'applyRedactions' })
    await port.answerNext({ dirtyPages: ['3-0', '9-0'] })
    await applying
    expect([useDocumentStore.getState().pageRevisions['3-0'], useDocumentStore.getState().pageRevisions['9-0']]).toEqual([1, 1])
    expect(useRedactionStore.getState().applying).toBe(false)
    expect(changes()).toBe(1)
  })

  it('backs out of the confirmation and reports a refusal', async () => {
    const { port, store, changes } = createStore()
    store.requestApply()
    store.cancel()
    await store.apply()
    expect(port.requests).toHaveLength(0)
    store.requestApply()
    const applying = store.apply()
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'nothing is marked for redaction' })
    await applying
    expect(useRedactionStore.getState()).toMatchObject({ applying: false, notice: 'nothing is marked for redaction' })
    expect(changes()).toBe(0)
    store.requestApply()
    expect(useRedactionStore.getState().notice).toBeNull()
  })
})
