import { PasswordPrompt } from '@/components/document/PasswordPrompt.tsx'
import { Hint } from '@/components/ui/Text.tsx'
import { HomeScreen } from '@/components/home/HomeScreen.tsx'
import { DocumentScroller } from '@/components/viewer/DocumentScroller.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function DocumentView() {
  const state = useDocumentStore((store) => store.document)
  if (state.phase === 'ready') {
    return (
      <section className="flex min-h-0 min-w-0 flex-1">
        <DocumentScroller key={state.session} document={state} />
      </section>
    )
  }
  return (
    <section className="flex min-h-0 min-w-0 flex-1 items-center justify-center">
      {state.phase === 'locked' ? (
        <PasswordPrompt name={state.name} wrongPassword={state.wrongPassword} />
      ) : state.phase === 'loading' ? (
        <Hint>Opening {state.name}</Hint>
      ) : (
        <HomeScreen />
      )}
    </section>
  )
}
