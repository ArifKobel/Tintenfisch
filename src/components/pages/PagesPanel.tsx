import { useState, type DragEvent, type KeyboardEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import { OrganizeToolbar } from '@/components/pages/OrganizeToolbar.tsx'
import { useOrganizeState } from '@/hooks/useOrganizeState.ts'
import { Thumbnail, type ThumbnailGesture } from '@/components/pages/Thumbnail.tsx'
import { SidePanel } from '@/components/ui/SidePanel.tsx'
import { useViewportState } from '@/hooks/useViewportState.ts'

interface Drag {
  pageIds: string[]
  before: string | null
}

export function PagesPanel() {
  const { organize, panels, viewport } = useApplication()
  const document = useDocumentState()
  const { selection } = useOrganizeState()
  const { currentPageId } = useViewportState()
  const [drag, setDrag] = useState<Drag | null>(null)
  if (document.phase !== 'ready') return null
  const order = document.pages.map((page) => page.id)

  function choose(pageId: string, { toggle, range }: ThumbnailGesture) {
    organize.select(pageId, { toggle, range })
    if (!toggle && !range) viewport.focusPage(pageId)
  }

  function startDrag(pageId: string, event: DragEvent) {
    const pageIds = selection.has(pageId) ? selection.inOrder(order) : [pageId]
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', pageIds.join(' '))
    setDrag({ pageIds, before: null })
  }

  function hover(pageId: string, lowerHalf: boolean) {
    if (!drag) return
    const before = lowerHalf ? (order[order.indexOf(pageId) + 1] ?? null) : pageId
    if (before !== drag.before) setDrag({ ...drag, before })
  }

  function drop() {
    if (drag) void organize.move(drag.pageIds, drag.before)
    setDrag(null)
  }

  function press(event: KeyboardEvent) {
    if ((event.key === 'Delete' || event.key === 'Backspace') && !(event.target instanceof HTMLInputElement)) {
      event.preventDefault()
      event.stopPropagation()
      void organize.deleteSelected()
    }
  }

  return (
    <SidePanel title="Pages" side="right" onClose={() => panels.showRight(null)} onKeyDown={press}>
      <OrganizeToolbar />
      <div className="thumbnail-list" onDragOver={(event) => drag && event.preventDefault()} onDrop={drop} onDragEnd={() => setDrag(null)}>
        {document.pages.map((page, index) => (
          <Thumbnail
            key={page.id}
            session={document.session}
            page={page}
            number={index + 1}
            current={page.id === currentPageId}
            selected={selection.has(page.id)}
            dropBefore={drag?.before === page.id}
            onChoose={(gesture) => choose(page.id, gesture)}
            onDragStart={(event) => startDrag(page.id, event)}
            onHover={(lowerHalf) => hover(page.id, lowerHalf)}
          />
        ))}
        {drag && drag.before === null && <div className="thumbnail-drop-line" />}
      </div>
    </SidePanel>
  )
}
