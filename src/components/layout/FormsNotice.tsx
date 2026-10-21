import { useFormsState } from '@/hooks/useFormsState.ts'

export function FormsNotice() {
  const { notice } = useFormsState()
  if (!notice) return null
  return (
    <span className="toast" role="status">
      {notice}
    </span>
  )
}
