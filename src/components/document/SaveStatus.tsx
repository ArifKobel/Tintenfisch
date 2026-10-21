import { Icon } from '@/components/ui/Icon.tsx'
import { useAutoSaveStore } from '@/stores/autoSaveStore.ts'

export function SaveStatus() {
  const { status, error } = useAutoSaveStore()
  if (status === 'idle') return null
  if (status === 'failed') {
    return (
      <span className="save-status save-status-failed" role="alert" title={error ?? undefined}>
        <Icon name="cloud-state-error" />
        <span className="save-status-label">Not saved</span>
      </span>
    )
  }
  const saved = status === 'saved'
  return (
    <span className="save-status" role="status">
      <Icon name={saved ? 'cloud-state-online' : 'cloud-state-in-progress'} />
      <span className="save-status-label">{saved ? 'Saved' : 'Saving…'}</span>
    </span>
  )
}
