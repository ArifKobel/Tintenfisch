import { useEffect, useRef, type KeyboardEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { useFindState } from '@find/useFindState.ts'

export function FindBar() {
  const { find, viewport } = useApplication()
  const document = useDocumentState()
  const { query, matches, current } = useFindState()
  const input = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const focus = (event: globalThis.KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
        event.preventDefault()
        input.current?.focus()
        input.current?.select()
      }
    }
    window.addEventListener('keydown', focus)
    return () => window.removeEventListener('keydown', focus)
  }, [])

  const matchPage = matches[current]?.pageId
  const matchTop = matches[current]?.rects[0]?.y ?? 0
  useEffect(() => {
    if (matchPage) viewport.focusPage(matchPage, matchTop)
  }, [viewport, matchPage, matchTop])

  if (document.phase !== 'ready') return null

  function step(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key !== 'Enter') return
    if (event.shiftKey) find.previous()
    else find.next()
  }

  return (
    <div className="find-bar" role="search">
      <input
        ref={input}
        type="search"
        placeholder="Find"
        value={query}
        onChange={(event) => void find.search(event.target.value)}
        onKeyDown={step}
        aria-label="Find in document"
      />
      <span className="find-count">{query ? `${matches.length === 0 ? 0 : current + 1} of ${matches.length}` : ''}</span>
      <button type="button" className="button" onClick={() => find.previous()} disabled={matches.length === 0} aria-label="Previous match">
        ‹
      </button>
      <button type="button" className="button" onClick={() => find.next()} disabled={matches.length === 0} aria-label="Next match">
        ›
      </button>
    </div>
  )
}
