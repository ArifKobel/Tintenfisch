import { Icon } from '@/components/ui/Icon.tsx'
import { useAutoSaveStore } from '@/stores/autoSaveStore.ts'

export function SaveStatus() {
  const { status, error } = useAutoSaveStore()
  if (status === 'idle') return null
  if (status === 'failed') {
    return (
      <span className="flex flex-none items-center gap-1 text-xs text-negative" role="alert" title={error ?? undefined}>
        <Icon name="cloud-state-error" className="size-4" />
        <span className="max-md:hidden">Not saved</span>
      </span>
    )
  }
  const saved = status === 'saved'
  return (
    <span className="flex flex-none items-center gap-1 text-xs text-ink-muted" role="status">
      <Icon name={saved ? 'cloud-state-online' : 'cloud-state-in-progress'} className="size-4" />
      <span className="max-md:hidden">{saved ? 'Saved' : 'Saving…'}</span>
    </span>
  )
}
