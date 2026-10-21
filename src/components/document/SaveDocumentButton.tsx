import { isTauri } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/Button.tsx'
import { Icon } from '@/components/ui/Icon.tsx'
import { FileDownload } from '@/services/FileDownload.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function SaveDocumentButton() {
  const state = useDocumentStore((store) => store.document)
  if (state.phase !== 'ready') return null
  const label = isTauri() ? 'Save as…' : 'Download'

  async function save() {
    if (state.phase !== 'ready') return
    await FileDownload.offer(state.name, await useDocumentStore.getState().save())
  }

  return (
    <Button look="dark" className="max-md:px-2" aria-label={label} onClick={save}>
      <Icon name={isTauri() ? 'save-floppy' : 'download'} />
      <span className="max-md:hidden">{label}</span>
    </Button>
  )
}
