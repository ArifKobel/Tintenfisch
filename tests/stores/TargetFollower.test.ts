import { describe, expect, it } from 'vitest'
import { TargetFollower } from '@/stores/TargetFollower.ts'
import { ViewportStore } from '@/stores/ViewportStore.ts'

describe('TargetFollower', () => {
  it('shows a page target at the top and opens only web and mail addresses', () => {
    const viewport = new ViewportStore()
    const opened: string[] = []
    const follower = new TargetFollower(viewport, (uri) => opened.push(uri))
    follower.follow({ kind: 'page', pageId: '4-0', top: 120 })
    expect(viewport.getSnapshot().focusRequest).toMatchObject({ pageId: '4-0', y: 120, atTop: true })
    follower.follow({ kind: 'page', pageId: '5-0', top: null })
    expect(viewport.getSnapshot().focusRequest).toMatchObject({ pageId: '5-0', y: 0 })
    follower.follow({ kind: 'uri', uri: ' https://example.org/ ' })
    follower.follow({ kind: 'uri', uri: 'mailto:someone@example.org' })
    follower.follow({ kind: 'uri', uri: 'javascript:alert(1)' })
    follower.follow({ kind: 'uri', uri: 'file:///etc/passwd' })
    expect(opened).toEqual(['https://example.org/', 'mailto:someone@example.org'])
  })

  it('names the view at the top of the window as a target', () => {
    const viewport = new ViewportStore()
    const follower = new TargetFollower(viewport, () => {})
    expect(follower.currentView()).toBeNull()
    viewport.reportViewTop('3-0', 240)
    expect(follower.currentView()).toEqual({ kind: 'page', pageId: '3-0', top: 240 })
  })
})
