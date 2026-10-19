import type { MouseEvent, PointerEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useToolState } from '@tools/useToolState.ts'
import { EditFrame } from '@editing/EditFrame.tsx'
import { EditInput } from '@editing/EditInput.tsx'
import { useEditingState } from '@editing/useEditingState.ts'
import type { SelectionUnit } from '@engine/protocol.ts'
import { useFindState } from '@find/useFindState.ts'
import { HighlightBoxes } from '@selection/HighlightBoxes.tsx'
import { usePageTextLines } from '@selection/usePageTextLines.ts'
import { useSelectionState } from '@selection/useSelectionState.ts'

const UNITS: SelectionUnit[] = ['character', 'word', 'paragraph']

export function TextLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { selection, editing, comments, images } = useApplication()
  const { tool } = useToolState()
  const selected = useSelectionState()
  const found = useFindState()
  const edited = useEditingState()
  const active = edited.active?.pageId === pageId ? edited.active : null
  const caret = active ? active.caret : undefined
  const lines = usePageTextLines(pageId)

  function pointInPoints(event: MouseEvent<HTMLDivElement>) {
    const bounds = event.currentTarget.getBoundingClientRect()
    return { x: (event.clientX - bounds.left) / cssScale, y: (event.clientY - bounds.top) / cssScale }
  }

  function capture(event: PointerEvent<HTMLDivElement>) {
    if (event.button === 0) event.currentTarget.setPointerCapture(event.pointerId)
  }

  function begin(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    const { x, y } = pointInPoints(event)
    if (edited.enabled) {
      event.preventDefault()
      selection.clear()
      if (tool === 'addText') {
        void editing.beginNew(pageId, x, y)
        return
      }
      const bounds = event.currentTarget.getBoundingClientRect()
      const foreground = images.foregroundAt(pageId, x, y, (bounds.width * bounds.height) / cssScale ** 2)
      if (foreground !== null) {
        void editing.commit()
        images.select({ pageId, index: foreground })
        return
      }
      void editing.begin(pageId, x, y).then((opened) => (opened ? images.select(null) : images.selectAt(pageId, x, y)))
      return
    }
    void selection.begin(pageId, x, y, UNITS[Math.min(Math.max(event.detail, 1), 3) - 1])
  }

  function extend(event: PointerEvent<HTMLDivElement>) {
    if ((event.buttons & 1) === 0 || edited.enabled) return
    const { x, y } = pointInPoints(event)
    void selection.extend(pageId, x, y)
  }

  async function finish(event: MouseEvent<HTMLDivElement>) {
    if ((tool !== 'highlight' && tool !== 'redactText') || event.button !== 0) return
    const { x, y } = pointInPoints(event)
    await selection.extend(pageId, x, y)
    const chosen = selection.getSnapshot()
    if (chosen?.pageId !== pageId || chosen.text.trim() === '') return
    selection.clear()
    await comments.add(pageId, {
      kind: tool === 'highlight' ? 'highlight' : 'redactText',
      anchor: [chosen.anchor.line, chosen.anchor.offset],
      focus: [chosen.focus.line, chosen.focus.offset],
    })
  }

  const current = found.matches[found.current]
  const pageMatches = found.matches.filter((match) => match.pageId === pageId && match !== current)
  return (
    <div className={`text-layer${tool === 'addText' ? ' text-layer-adding' : ''}`} onPointerDown={capture} onMouseDown={begin} onPointerMove={extend} onMouseUp={(event) => void finish(event)}>
      <HighlightBoxes boxes={lines} cssScale={cssScale} className="text-line-region" />
      <HighlightBoxes boxes={pageMatches.flatMap((match) => match.rects)} cssScale={cssScale} className="find-match" />
      {current?.pageId === pageId && <HighlightBoxes boxes={current.rects} cssScale={cssScale} className="find-match find-match-current" />}
      {selected?.pageId === pageId && <HighlightBoxes boxes={selected.rects} cssScale={cssScale} className="text-selection" />}
      {active && <EditFrame frame={active.frame} cssScale={cssScale} />}
      {active && <HighlightBoxes boxes={active.selection} cssScale={cssScale} className="text-selection" />}
      {caret && active?.selection.length === 0 && <HighlightBoxes boxes={[caret]} cssScale={cssScale} className="edit-caret" />}
      {caret !== undefined && <EditInput caret={caret} cssScale={cssScale} />}
    </div>
  )
}
