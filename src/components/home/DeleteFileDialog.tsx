import { useLibraryStore } from '@/stores/libraryStore.ts'

export function DeleteFileDialog() {
  const { deleting, askToDelete, confirmDelete } = useLibraryStore()
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
          <button type="button" className="button" autoFocus onClick={() => askToDelete(null)}>
            Cancel
          </button>
          <button type="button" className="button button-danger" onClick={() => void confirmDelete()}>
            Delete
          </button>
        </div>
      </div>
    </div>
  )
}
