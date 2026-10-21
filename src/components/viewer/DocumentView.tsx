import { PasswordPrompt } from '@/components/document/PasswordPrompt.tsx'
import { HomeScreen } from '@/components/home/HomeScreen.tsx'
import { DocumentScroller } from '@/components/viewer/DocumentScroller.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function DocumentView() {
  const state = useDocumentStore((store) => store.document)
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
        <HomeScreen />
      )}
    </section>
  )
}
