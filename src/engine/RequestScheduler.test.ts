import { describe, expect, it } from 'vitest'
import type { Priority } from '@engine/protocol.ts'
import { RequestScheduler } from '@engine/RequestScheduler.ts'

const request = (id: number, priority: Priority, group?: string) => ({ id, priority, group })

describe('RequestScheduler', () => {
  it('hands out interactive work first and keeps arrival order within a priority', () => {
    const scheduler = new RequestScheduler<ReturnType<typeof request>>()
    scheduler.enqueue(request(1, 'prefetch'))
    scheduler.enqueue(request(2, 'visible'))
    scheduler.enqueue(request(3, 'interactive'))
    scheduler.enqueue(request(4, 'visible'))
    expect([1, 2, 3, 4].map(() => scheduler.next()?.id)).toEqual([3, 2, 4, 1])
    expect(scheduler.next()).toBeUndefined()
  })

  it('replaces queued requests of the same group', () => {
    const scheduler = new RequestScheduler<ReturnType<typeof request>>()
    scheduler.enqueue(request(1, 'visible', 'page-3'))
    scheduler.enqueue(request(2, 'visible', 'page-4'))
    const superseded = scheduler.enqueue(request(3, 'interactive', 'page-3'))
    expect(superseded.map((entry) => entry.id)).toEqual([1])
    expect(scheduler.size).toBe(2)
  })

  it('removes cancelled requests wherever they are queued', () => {
    const scheduler = new RequestScheduler<ReturnType<typeof request>>()
    scheduler.enqueue(request(1, 'prefetch'))
    scheduler.enqueue(request(2, 'interactive'))
    expect(scheduler.cancel([1, 99]).map((entry) => entry.id)).toEqual([1])
    expect(scheduler.next()?.id).toBe(2)
    expect(scheduler.size).toBe(0)
  })
})
