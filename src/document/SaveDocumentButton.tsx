import { useApplication } from '@app/ApplicationContext.ts'
import { FileDownload } from '@document/FileDownload.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { Icon } from '@ui/Icon.tsx'

export function SaveDocumentButton() {
  const { documents } = useApplication()
  const state = useDocumentState()
  if (state.phase !== 'ready') return null

  async function save() {
    if (state.phase !== 'ready') return
    FileDownload.offer(state.name, await documents.save())
  }

  return (
    <button type="button" className="button button-dark" aria-label="Download" onClick={save}>
      <Icon name="download" />
      <span className="button-label">Download</span>
    </button>
  )
}
