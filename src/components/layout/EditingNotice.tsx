import { useEditingState } from '@/hooks/useEditingState.ts'

export function EditingNotice() {
  const { notice } = useEditingState()
  if (!notice) return null
  return (
    <span className="toast" role="status">
      {notice}
    </span>
  )
}
