import { describe, expect, it } from 'vitest'
import { PanelStore } from '@shell/PanelStore.ts'

describe('PanelStore', () => {
  it('keeps one panel open on each side and always shows a left panel', () => {
    const panels = new PanelStore()
    expect(panels.getSnapshot().left).toBe('tools')
    panels.toggleRight('comments')
    panels.toggleRight('pages')
    expect(panels.getSnapshot()).toEqual({ left: 'tools', right: 'pages', sheet: false })
    panels.toggleRight('pages')
    panels.showLeft('edit')
    expect(panels.getSnapshot()).toEqual({ left: 'edit', right: null, sheet: false })
  })

  it('tells listeners only about real changes', () => {
    const panels = new PanelStore()
    let calls = 0
    panels.subscribe(() => calls++)
    panels.showRight('bookmarks')
    panels.showRight('bookmarks')
    expect(calls).toBe(1)
  })

  it('closes the sheet when a right panel takes over', () => {
    const panels = new PanelStore()
    panels.openSheet()
    expect(panels.getSnapshot().sheet).toBe(true)
    panels.showRight('comments')
    expect(panels.getSnapshot().sheet).toBe(false)
  })
})
