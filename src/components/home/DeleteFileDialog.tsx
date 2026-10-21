import { useLibraryStore } from '@/stores/libraryStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Dialog, DialogActions } from '@/components/ui/Dialog.tsx'

export function DeleteFileDialog() {
  const { deleting, askToDelete, confirmDelete } = useLibraryStore()
  if (!deleting) return null
  return (
    <Dialog title="Delete file?" titleId="delete-title" role="alertdialog">
        <p className="text-ink-body">
          {deleting.name} will be removed from this browser for good. A downloaded copy elsewhere stays untouched.
        </p>
        <DialogActions>
          <Button autoFocus onClick={() => askToDelete(null)}>
            Cancel
          </Button>
          <Button look="danger" onClick={() => void confirmDelete()}>
            Delete
          </Button>
        </DialogActions>
    </Dialog>
  )
}
