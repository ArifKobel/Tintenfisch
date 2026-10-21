import { useState, type FormEvent } from 'react'
import { useLinksStore, type LinkDraft } from '@/stores/linksStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Dialog, DialogActions, DialogChoice } from '@/components/ui/Dialog.tsx'
import { Input } from '@/components/ui/Field.tsx'

const SCHEME = /^[a-z][a-z0-9+.-]*:/i

export function LinkForm({ draft }: { draft: LinkDraft }) {
  const links = useLinksStore()
  const document = useDocumentStore((state) => state.document)
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
    <Dialog title={draft.linkId ? 'Edit link' : 'New link'} label={draft.linkId ? 'Edit link' : 'New link'} onSubmit={submit}>
        <DialogChoice>
          <input type="radio" name="link-kind" checked={kind === 'page'} onChange={() => setKind('page')} />
          Go to page
          <Input
            className="w-[5em]"
            type="number"
            min={1}
            max={pages.length}
            aria-label="Page number"
            value={pageNumber}
            onFocus={() => setKind('page')}
            onChange={(event) => setPageNumber(event.target.value)}
          />
          of {pages.length}
        </DialogChoice>
        <DialogChoice>
          <input type="radio" name="link-kind" checked={kind === 'uri'} onChange={() => setKind('uri')} />
          Open web address
        </DialogChoice>
        <Input className="ml-[22px]" aria-label="Web address" placeholder="https://" value={uri} onFocus={() => setKind('uri')} onChange={(event) => setUri(event.target.value)} />
        <DialogActions>
          {draft.linkId && (
            <Button onClick={() => void links.removeSelected()}>
              Delete
            </Button>
          )}
          <Button onClick={() => links.closeDraft()}>
            Cancel
          </Button>
          <Button type="submit" look="primary" disabled={!ready}>
            Save
          </Button>
        </DialogActions>
    </Dialog>
  )
}
