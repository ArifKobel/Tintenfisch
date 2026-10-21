import { Icon } from '@/components/ui/Icon.tsx'
import { FileDownload } from '@/services/FileDownload.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function SaveDocumentButton() {
  const state = useDocumentStore((store) => store.document)
  if (state.phase !== 'ready') return null

  async function save() {
    if (state.phase !== 'ready') return
    FileDownload.offer(state.name, await useDocumentStore.getState().save())
  }

  return (
    <button type="button" className="button button-dark" aria-label="Download" onClick={save}>
      <Icon name="download" />
      <span className="button-label">Download</span>
    </button>
  )
}
