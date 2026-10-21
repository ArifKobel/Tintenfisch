import { describe, expect, it } from 'vitest'
import { EngineClient } from '@/services/EngineClient.ts'
import { FindStore } from '@/stores/FindStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const matches = [
  { pageId: '3-0', rects: [{ x: 1, y: 2, width: 2, height: 2, angle: 0 }] },
  { pageId: '5-0', rects: [{ x: 5, y: 6, width: 2, height: 2, angle: 0 }] },
]

describe('FindStore', () => {
  it('searches the document and steps through matches in both directions with wrap around', async () => {
    const port = new FakeEnginePort()
    const find = new FindStore(new EngineClient(port))
    const searching = find.search('stream')
    await port.answerNext(matches)
    await searching
    expect(find.getSnapshot()).toMatchObject({ query: 'stream', current: 0 })
    find.next()
    expect(find.currentMatch?.pageId).toBe('5-0')
    find.next()
    expect(find.currentMatch?.pageId).toBe('3-0')
    find.previous()
    expect(find.currentMatch?.pageId).toBe('5-0')
  })

  it('refreshes the same query after an edit and keeps the current match where it can', async () => {
    const port = new FakeEnginePort()
    const find = new FindStore(new EngineClient(port))
    const searching = find.search('stream')
    await port.answerNext(matches)
    await searching
    find.next()
    const refreshing = find.refresh()
    expect(port.requests[port.requests.length - 1]).toMatchObject({ kind: 'find', query: 'stream' })
    await port.answerNext(matches.slice(0, 1))
    await refreshing
    expect(find.getSnapshot()).toMatchObject({ query: 'stream', current: 0, matches: matches.slice(0, 1) })
  })

  it('clears for an empty query without asking the engine', async () => {
    const port = new FakeEnginePort()
    const find = new FindStore(new EngineClient(port))
    await find.search('   ')
    expect(port.requests).toHaveLength(0)
    expect(find.getSnapshot().matches).toEqual([])
    find.next()
    expect(find.currentMatch).toBeUndefined()
  })
})
