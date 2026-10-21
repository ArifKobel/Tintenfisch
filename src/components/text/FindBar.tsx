import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import { useFindState } from '@/hooks/useFindState.ts'
import { Icon } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'

export function FindBar() {
  const { find, viewport } = useApplication()
  const document = useDocumentState()
  const { query, matches, current } = useFindState()
  const [open, setOpen] = useState(false)
  const input = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const focus = (event: globalThis.KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
        event.preventDefault()
        setOpen(true)
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
  if (!open) return <IconButton icon="search" label="Find text" onClick={() => setOpen(true)} />

  function close() {
    void find.search('')
    setOpen(false)
  }

  function step(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Escape') close()
    if (event.key !== 'Enter') return
    if (event.shiftKey) find.previous()
    else find.next()
  }

  return (
    <div className="find-bar" role="search">
      <label className="find-field">
        <Icon name="search" />
        <input
          ref={input}
          type="search"
          placeholder="Find text"
          value={query}
          onChange={(event) => void find.search(event.target.value)}
          onKeyDown={step}
          aria-label="Find in document"
          autoFocus
        />
        <span className="find-count">{query ? `${matches.length === 0 ? 0 : current + 1} of ${matches.length}` : ''}</span>
      </label>
      <IconButton icon="chevron-up" label="Previous match" disabled={matches.length === 0} onClick={() => find.previous()} />
      <IconButton icon="chevron-down" label="Next match" disabled={matches.length === 0} onClick={() => find.next()} />
      <IconButton icon="close" label="Close find" onClick={close} />
    </div>
  )
}
