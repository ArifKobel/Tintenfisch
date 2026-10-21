import { useState } from 'react'
import { IconButton } from '@/components/ui/IconButton.tsx'
import type { ReadyDocument } from '@/stores/documentStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'

export function PageNavigator({ document }: { document: ReadyDocument }) {
  const { currentPageId, showAtTop } = useViewportStore()
  const [typed, setTyped] = useState<string | null>(null)
  const index = Math.max(0, document.pages.findIndex((page) => page.id === currentPageId))
  const count = document.pages.length

  function show(target: number) {
    const page = document.pages[Math.min(count - 1, Math.max(0, target))]
    if (page) showAtTop(page.id)
  }

  function commit() {
    if (typed?.trim() && Number.isFinite(Number(typed))) show(Number(typed) - 1)
    setTyped(null)
  }

  return (
    <form
      className="page-navigator"
      onSubmit={(event) => {
        event.preventDefault()
        commit()
      }}
    >
      <input
        className="page-number"
        inputMode="numeric"
        aria-label={`Page number, ${count} pages`}
        value={typed ?? String(index + 1)}
        onFocus={(event) => event.currentTarget.select()}
        onChange={(event) => setTyped(event.target.value)}
        onBlur={commit}
      />
      <span className="page-count">{count}</span>
      <IconButton icon="chevron-up" label="Previous page" tooltip="left" disabled={index === 0} onClick={() => show(index - 1)} />
      <IconButton icon="chevron-down" label="Next page" tooltip="left" disabled={index >= count - 1} onClick={() => show(index + 1)} />
    </form>
  )
}
