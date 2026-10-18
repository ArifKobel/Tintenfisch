import { useFormsState } from '@forms/useFormsState.ts'

export function FormsNotice() {
  const { notice } = useFormsState()
  if (!notice) return null
  return (
    <span className="edit-notice" role="status">
      {notice}
    </span>
  )
}
