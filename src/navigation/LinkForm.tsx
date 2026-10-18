import { useState, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import type { LinkDraft } from '@navigation/LinksStore.ts'

const SCHEME = /^[a-z][a-z0-9+.-]*:/i

export function LinkForm({ draft }: { draft: LinkDraft }) {
  const { links } = useApplication()
  const document = useDocumentState()
  const pages = document.phase === 'ready' ? document.pages : []
  const target = draft.target
  const startPage = target?.kind === 'page' ? pages.findIndex((page) => page.id === target.pageId) + 1 : 1
  const [kind, setKind] = useState<'page' | 'uri'>(target?.kind ?? 'page')
  const [pageNumber, setPageNumber] = useState(String(Math.max(startPage, 1)))
  const [uri, setUri] = useState(target?.kind === 'uri' ? target.uri : '')
  const chosenPage = pages[Number(pageNumber) - 1]
  const address = uri.trim()
  const ready = kind === 'page' ? chosenPage !== undefined : address !== ''

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready) return
    if (kind === 'uri') {
      void links.save({ kind: 'uri', uri: SCHEME.test(address) ? address : `https://${address}` })
      return
    }
    const top = target?.kind === 'page' && target.pageId === chosenPage.id ? target.top : null
    void links.save({ kind: 'page', pageId: chosenPage.id, top })
  }

  return (
    <div className="link-backdrop">
      <form className="link-dialog" role="dialog" aria-modal="true" aria-label={draft.linkId ? 'Edit link' : 'New link'} onSubmit={submit}>
        <h2 className="link-title">{draft.linkId ? 'Edit link' : 'New link'}</h2>
        <label className="link-choice">
          <input type="radio" name="link-kind" checked={kind === 'page'} onChange={() => setKind('page')} />
          Go to page
          <input
            className="link-page"
            type="number"
            min={1}
            max={pages.length}
            aria-label="Page number"
            value={pageNumber}
            onFocus={() => setKind('page')}
            onChange={(event) => setPageNumber(event.target.value)}
          />
          of {pages.length}
        </label>
        <label className="link-choice">
          <input type="radio" name="link-kind" checked={kind === 'uri'} onChange={() => setKind('uri')} />
          Open web address
        </label>
        <input className="link-address" aria-label="Web address" placeholder="https://" value={uri} onFocus={() => setKind('uri')} onChange={(event) => setUri(event.target.value)} />
        <div className="link-actions">
          {draft.linkId && (
            <button type="button" className="button" onClick={() => void links.removeSelected()}>
              Delete
            </button>
          )}
          <button type="button" className="button" onClick={() => links.closeDraft()}>
            Cancel
          </button>
          <button type="submit" className="button button-primary" disabled={!ready}>
            Save
          </button>
        </div>
      </form>
    </div>
  )
}
