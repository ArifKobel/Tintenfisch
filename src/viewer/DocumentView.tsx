import { PasswordPrompt } from '@document/PasswordPrompt.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { DocumentScroller } from '@viewer/DocumentScroller.tsx'

export function DocumentView() {
  const state = useDocumentState()
  if (state.phase === 'locked') {
    return (
      <section className="document-view document-view-empty">
        <PasswordPrompt name={state.name} wrongPassword={state.wrongPassword} />
      </section>
    )
  }
  if (state.phase !== 'ready') {
    return (
      <section className="document-view document-view-empty">
        <p>{state.phase === 'loading' ? `Opening ${state.name}` : 'Open a PDF to start.'}</p>
      </section>
    )
  }
  return (
    <section className="document-view">
      <DocumentScroller key={state.session} document={state} />
    </section>
  )
}
