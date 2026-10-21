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
      <div className="flex min-w-0 items-baseline gap-2" role="status">
        <span className="truncate font-bold text-ink-strong">{state.name}</span>
        <span className={`flex-none text-xs ${state.phase === 'failed' ? 'text-negative' : 'text-ink-muted'}`}>{detail}</span>
      </div>
    )
  }
  const facts = [
    `${state.pageCount.toLocaleString()} ${state.pageCount === 1 ? 'page' : 'pages'}`,
    `${state.objectCount.toLocaleString()} objects`,
    ...(state.repaired ? ['repaired'] : []),
  ]
  return (
    <div className="flex min-w-0 items-baseline gap-2" role="status" title={facts.join(', ')}>
      <span className="truncate font-bold text-ink-strong">{state.name}</span>
      <span className="flex-none text-xs text-ink-muted max-md:hidden">PDF {state.version}</span>
      {pending && (
        <span className="flex self-center text-ink-muted" title={pending}>
          <Icon name="lock" className="size-4" />
        </span>
      )}
    </div>
  )
}
