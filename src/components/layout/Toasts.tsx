import { EditingNotice } from '@/components/layout/EditingNotice.tsx'
import { FilesNotice } from '@/components/layout/FilesNotice.tsx'
import { FormsNotice } from '@/components/layout/FormsNotice.tsx'

export function Toasts() {
  return (
    <div className="toasts" aria-live="polite">
      <EditingNotice />
      <FormsNotice />
      <FilesNotice />
    </div>
  )
}
