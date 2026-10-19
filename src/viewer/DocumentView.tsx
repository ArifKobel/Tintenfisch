import { PasswordPrompt } from '@document/PasswordPrompt.tsx'
import { StartScreen } from '@document/StartScreen.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { DocumentScroller } from '@viewer/DocumentScroller.tsx'

export function DocumentView() {
  const state = useDocumentState()
  if (state.phase === 'ready') {
    return (
      <section className="document-view">
        <DocumentScroller key={state.session} document={state} />
      </section>
    )
  }
  return (
    <section className="document-view document-view-empty">
      {state.phase === 'locked' ? (
        <PasswordPrompt name={state.name} wrongPassword={state.wrongPassword} />
      ) : state.phase === 'loading' ? (
        <p className="hint">Opening {state.name}</p>
      ) : (
        <StartScreen />
      )}
    </section>
  )
}
