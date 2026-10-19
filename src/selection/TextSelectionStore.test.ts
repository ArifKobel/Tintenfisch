import { describe, expect, it } from 'vitest'
import { EngineClient } from '@engine/EngineClient.ts'
import { TextSelectionStore } from '@selection/TextSelectionStore.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

const selection = (from: number, to: number) => ({
  anchor: { line: 0, offset: from },
  focus: { line: 0, offset: to },
  rects: [{ x: 0, y: 0, width: 10, height: 10, angle: 0 }],
  text: 'word'.slice(from, to),
})

function createStore() {
  const port = new FakeEnginePort()
  return { port, store: new TextSelectionStore(new EngineClient(port)) }
}

describe('TextSelectionStore', () => {
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
    expect(store.getSnapshot()).toMatchObject({ pageId: '3-0', text: 'wor' })
    expect(store.text).toBe('wor')
  })

  it('clears when the press lands on no text and ignores drags on other pages', async () => {
    const { port, store } = createStore()
    const begin = store.begin('3-0', 500, 700, 'word')
    await port.answerNext(null)
    await begin
    expect(store.getSnapshot()).toBeNull()
    await store.extend('4-0', 1, 1)
    expect(port.requests).toHaveLength(1)
  })

  it('drops answers that a newer gesture overtook', async () => {
    const { port, store } = createStore()
    const first = store.begin('3-0', 10, 20, 'character')
    const firstHit = port.lastId
    const second = store.begin('3-0', 30, 20, 'word')
    port.answer({ id: firstHit, ok: true, result: { line: 0, offset: 0 }, revision: 0 })
    await first
    await port.answerNext({ line: 0, offset: 2 })
    await port.answerNext(selection(0, 4))
    await second
    expect(store.getSnapshot()).toMatchObject({ unit: 'word', text: 'word' })
  })
})
