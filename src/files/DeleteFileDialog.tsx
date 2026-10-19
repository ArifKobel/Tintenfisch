import { useApplication } from '@app/ApplicationContext.ts'
import { useFilesState } from '@files/useFilesState.ts'

export function DeleteFileDialog() {
  const { files } = useApplication()
  const { deleting } = useFilesState()
  if (!deleting) return null
  return (
    <div className="dialog-backdrop">
      <div className="dialog" role="alertdialog" aria-modal="true" aria-labelledby="delete-title">
        <h2 id="delete-title" className="dialog-title">
          Delete file?
        </h2>
        <p className="dialog-hint">
          {deleting.name} will be removed from this browser for good. A downloaded copy elsewhere stays untouched.
        </p>
        <div className="dialog-actions">
          <button type="button" className="button" autoFocus onClick={() => files.askToDelete(null)}>
            Cancel
          </button>
          <button type="button" className="button button-danger" onClick={() => void files.confirmDelete()}>
            Delete
          </button>
        </div>
      </div>
    </div>
  )
}
