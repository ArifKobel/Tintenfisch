import { Button } from '@/components/ui/Button.tsx'
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
    <Button look="dark" className="max-md:px-2" aria-label="Download" onClick={save}>
      <Icon name="download" />
      <span className="max-md:hidden">Download</span>
    </Button>
  )
}
