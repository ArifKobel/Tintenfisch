import { useAutoSaveState } from '@files/useAutoSaveState.ts'
import { Icon } from '@ui/Icon.tsx'

export function SaveStatus() {
  const { status, error } = useAutoSaveState()
  if (status === 'idle') return null
  if (status === 'failed') {
    return (
      <span className="save-status save-status-failed" role="alert" title={error ?? undefined}>
        <Icon name="cloud-state-error" />
        Not saved
      </span>
    )
  }
  const saved = status === 'saved'
  return (
    <span className="save-status" role="status">
      <Icon name={saved ? 'cloud-state-online' : 'cloud-state-in-progress'} />
      {saved ? 'Saved' : 'Saving…'}
    </span>
  )
}
