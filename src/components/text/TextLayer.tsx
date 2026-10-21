import { useRef, type MouseEvent, type PointerEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { useToolState } from '@/hooks/useToolState.ts'
import { EditFrame } from '@/components/editing/EditFrame.tsx'
import { EditInput } from '@/components/editing/EditInput.tsx'
import { useEditingState } from '@/hooks/useEditingState.ts'
import type { SelectionUnit } from '@/types/protocol.ts'
import { useFindState } from '@/hooks/useFindState.ts'
import { HighlightBoxes } from '@/components/text/HighlightBoxes.tsx'
import { TextBlocks } from '@/lib/TextBlocks.ts'
import { usePageTextBlocks } from '@/hooks/usePageTextBlocks.ts'
import { useSelectionState } from '@/hooks/useSelectionState.ts'

const UNITS: SelectionUnit[] = ['character', 'word', 'paragraph']

export function TextLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { selection, editing, comments, images, tools } = useApplication()
  const { tool } = useToolState()
  const selected = useSelectionState()
  const found = useFindState()
  const edited = useEditingState()
  const active = edited.active?.pageId === pageId ? edited.active : null
  const caret = active ? active.caret : undefined
  const blocks = usePageTextBlocks(pageId)
  const dragging = useRef<number | null>(null)

  function pointInPoints(event: MouseEvent<HTMLDivElement>) {
    const bounds = event.currentTarget.getBoundingClientRect()
    return { x: (event.clientX - bounds.left) / cssScale, y: (event.clientY - bounds.top) / cssScale }
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
      const clicks = event.detail
      if (active && TextBlocks.contain([active.frame], x, y)) {
        void editing.press(pageId, x, y, clicks, event.shiftKey)
        return
      }
      if (!TextBlocks.contain(blocks, x, y)) {
        void editing.commit()
        images.selectAt(pageId, x, y)
        return
      }
      void editing.begin(pageId, x, y).then((opened) => {
        if (!opened) return images.selectAt(pageId, x, y)
        images.select(null)
        if (clicks >= 2) void editing.press(pageId, x, y, clicks, false)
      })
      return
    }
    if (tool === 'select' && event.detail === 2 && TextBlocks.contain(blocks, x, y)) {
      event.preventDefault()
      selection.clear()
      tools.choose('edit')
      void editing.begin(pageId, x, y)
      return
    }
    void selection.begin(pageId, x, y, UNITS[Math.min(Math.max(event.detail, 1), 3) - 1])
  }

  function press(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || !(event.target instanceof Element)) return
    event.target.setPointerCapture(event.pointerId)
    dragging.current = event.pointerId
  }

  function release(event: PointerEvent<HTMLDivElement>) {
    if (dragging.current === event.pointerId) dragging.current = null
  }

  function extend(event: PointerEvent<HTMLDivElement>) {
    if (dragging.current !== event.pointerId || (event.buttons & 1) === 0) return
    const { x, y } = pointInPoints(event)
    if (!edited.enabled) void selection.extend(pageId, x, y)
    else if (active && tool !== 'addText') void editing.dragTo(pageId, x, y)
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
    <div className={`text-layer${tool === 'addText' ? ' text-layer-adding' : ''}`} onPointerDown={press} onMouseDown={begin} onPointerMove={extend} onPointerUp={release} onLostPointerCapture={release} onMouseUp={(event) => void finish(event)}>
      <HighlightBoxes boxes={blocks} cssScale={cssScale} className="text-block-region" />
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
