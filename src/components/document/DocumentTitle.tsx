import { Icon } from '@/components/ui/Icon.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useProtectionStore } from '@/stores/protectionStore.ts'

export function DocumentTitle() {
  const state = useDocumentStore((store) => store.document)
  const pending = useProtectionStore((store) => store.pending)
  if (state.phase === 'empty') return null
  if (state.phase !== 'ready') {
    const detail = state.phase === 'loading' ? 'Opening' : state.phase === 'locked' ? 'Password required' : state.message
    return (
      <div className={`document-title document-title-${state.phase}`} role="status">
        <span className="document-name">{state.name}</span>
        <span className="document-detail">{detail}</span>
      </div>
    )
  }
  const facts = [
    `${state.pageCount.toLocaleString()} ${state.pageCount === 1 ? 'page' : 'pages'}`,
    `${state.objectCount.toLocaleString()} objects`,
    ...(state.repaired ? ['repaired'] : []),
  ]
  return (
    <div className="document-title" role="status" title={facts.join(', ')}>
      <span className="document-name">{state.name}</span>
      <span className="document-kind">PDF {state.version}</span>
      {pending && (
        <span className="document-pending" title={pending}>
          <Icon name="lock" />
        </span>
      )}
    </div>
  )
}
