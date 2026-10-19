import { describe, expect, it } from 'vitest'
import { PanelStore } from '@shell/PanelStore.ts'

describe('PanelStore', () => {
  it('keeps one panel open on each side and closes it when toggled again', () => {
    const panels = new PanelStore()
    panels.toggleRight('comments')
    panels.toggleLeft('tools')
    panels.toggleRight('pages')
    expect(panels.getSnapshot()).toEqual({ left: 'tools', right: 'pages' })
    panels.toggleRight('pages')
    panels.toggleLeft('edit')
    expect(panels.getSnapshot()).toEqual({ left: 'edit', right: null })
  })

  it('tells listeners only about real changes', () => {
    const panels = new PanelStore()
    let calls = 0
    panels.subscribe(() => calls++)
    panels.showRight('bookmarks')
    panels.showRight('bookmarks')
    expect(calls).toBe(1)
  })
})
