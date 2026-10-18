import type { Priority, Scheduling } from '@engine/protocol.ts'

const ORDER: Priority[] = ['interactive', 'visible', 'prefetch']

export type Schedulable = Scheduling & { id: number }

export class RequestScheduler<T extends Schedulable> {
  readonly #queues = new Map<Priority, T[]>(ORDER.map((priority) => [priority, []]))

  get size(): number {
    return ORDER.reduce((total, priority) => total + this.#queue(priority).length, 0)
  }

  enqueue(request: T): T[] {
    const superseded = request.group === undefined ? [] : this.#remove((queued) => queued.group === request.group)
    this.#queue(request.priority).push(request)
    return superseded
  }

  cancel(ids: number[]): T[] {
    const wanted = new Set(ids)
    return this.#remove((queued) => wanted.has(queued.id))
  }

  next(): T | undefined {
    for (const priority of ORDER) {
      const request = this.#queue(priority).shift()
      if (request) return request
    }
    return undefined
  }

  #queue(priority: Priority): T[] {
    return this.#queues.get(priority)!
  }

  #remove(matches: (request: T) => boolean): T[] {
    const removed: T[] = []
    for (const priority of ORDER) {
      const queue = this.#queue(priority)
      const kept = queue.filter((request) => {
        if (!matches(request)) return true
        removed.push(request)
        return false
      })
      this.#queues.set(priority, kept)
    }
    return removed
  }
}
