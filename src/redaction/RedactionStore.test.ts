import { describe, expect, it } from 'vitest'
import { EngineClient } from '@engine/EngineClient.ts'
import { RedactionStore } from '@redaction/RedactionStore.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'
import { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

function createStore() {
  const port = new FakeEnginePort()
  const revisions = new PageRevisionStore()
  const store = new RedactionStore(new EngineClient(port), revisions)
  let changes = 0
  store.onDocumentChange(() => changes++)
  return { port, revisions, store, changes: () => changes }
}

describe('RedactionStore', () => {
  it('applies only after confirming, then redraws the changed pages and announces the change', async () => {
    const { port, revisions, store, changes } = createStore()
    await store.apply()
    expect(port.requests).toHaveLength(0)
    store.requestApply()
    expect(store.getSnapshot().confirming).toBe(true)
    const applying = store.apply()
    expect(store.getSnapshot()).toMatchObject({ confirming: false, applying: true })
    expect(port.requests[0]).toMatchObject({ kind: 'applyRedactions' })
    await port.answerNext({ dirtyPages: ['3-0', '9-0'] })
    await applying
    expect([revisions.revisionOf('3-0'), revisions.revisionOf('9-0')]).toEqual([1, 1])
    expect(store.getSnapshot().applying).toBe(false)
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
    expect(store.getSnapshot()).toMatchObject({ applying: false, notice: 'nothing is marked for redaction' })
    expect(changes()).toBe(0)
    store.requestApply()
    expect(store.getSnapshot().notice).toBeNull()
  })
})
