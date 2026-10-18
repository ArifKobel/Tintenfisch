import { describe, expect, it } from 'vitest'
import { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

describe('PageRevisionStore', () => {
  it('advances only the touched pages and tells its listeners', () => {
    const revisions = new PageRevisionStore()
    let notified = 0
    revisions.subscribe(() => notified++)
    revisions.touch(['3-0', '3-0', '7-0'])
    expect(revisions.revisionOf('3-0')).toBe(2)
    expect(revisions.revisionOf('7-0')).toBe(1)
    expect(revisions.revisionOf('9-0')).toBe(0)
    expect(notified).toBe(1)
  })
})
