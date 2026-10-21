import { describe, expect, it } from 'vitest'
import { useSelectionStore } from '@/stores/selectionStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const selection = (from: number, to: number) => ({
  anchor: { line: 0, offset: from },
  focus: { line: 0, offset: to },
  rects: [{ x: 0, y: 0, width: 10, height: 10, angle: 0 }],
  text: 'word'.slice(from, to),
})

function createStore() {
  const port = FakeEnginePort.attach()
  return { port, store: useSelectionStore.getState() }
}

describe('selectionStore', () => {
  it('anchors where the press lands and extends while dragging', async () => {
    const { port, store } = createStore()
    const begin = store.begin('3-0', 10, 20, 'character')
    await port.answerNext({ line: 0, offset: 0 })
    await port.answerNext(selection(0, 0))
    await begin
    const extend = store.extend('3-0', 40, 20)
    await port.answerNext({ line: 0, offset: 3 })
    expect(port.requests[port.requests.length - 1]).toMatchObject({ kind: 'selectText', anchor: { offset: 0 }, focus: { offset: 3 } })
    await port.answerNext(selection(0, 3))
    await extend
    expect(useSelectionStore.getState().selection).toMatchObject({ pageId: '3-0', text: 'wor' })
    expect(useSelectionStore.getState().selection?.text).toBe('wor')
  })

  it('clears when the press lands on no text and ignores drags on other pages', async () => {
    const { port, store } = createStore()
    const begin = store.begin('3-0', 500, 700, 'word')
    await port.answerNext(null)
    await begin
    expect(useSelectionStore.getState().selection).toBeNull()
    await store.extend('4-0', 1, 1)
    expect(port.requests).toHaveLength(1)
  })

  it('drops answers that a newer gesture overtook', async () => {
    const { port, store } = createStore()
    const first = store.begin('3-0', 10, 20, 'character')
    const firstHit = port.lastId
    const second = store.begin('3-0', 30, 20, 'word')
    port.answer({ id: firstHit, ok: true, result: { line: 0, offset: 0 }, revision: '' })
    await first
    await port.answerNext({ line: 0, offset: 2 })
    await port.answerNext(selection(0, 4))
    await second
    expect(useSelectionStore.getState().selection).toMatchObject({ unit: 'word', text: 'word' })
  })
})
