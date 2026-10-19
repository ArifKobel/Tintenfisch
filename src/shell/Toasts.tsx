import { EditingNotice } from '@editing/EditingNotice.tsx'
import { FilesNotice } from '@files/FilesNotice.tsx'
import { FormsNotice } from '@forms/FormsNotice.tsx'

export function Toasts() {
  return (
    <div className="toasts" aria-live="polite">
      <EditingNotice />
      <FormsNotice />
      <FilesNotice />
    </div>
  )
}
