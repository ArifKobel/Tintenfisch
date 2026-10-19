import { useFilesState } from '@files/useFilesState.ts'

export function FilesNotice() {
  const { notice } = useFilesState()
  if (!notice) return null
  return (
    <span className="toast" role="alert">
      {notice}
    </span>
  )
}
