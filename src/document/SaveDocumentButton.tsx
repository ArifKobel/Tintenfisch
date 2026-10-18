import { useApplication } from '@app/ApplicationContext.ts'
import { FileDownload } from '@document/FileDownload.ts'
import { useDocumentState } from '@document/useDocumentState.ts'

export function SaveDocumentButton() {
  const { documents } = useApplication()
  const state = useDocumentState()

  async function save() {
    if (state.phase !== 'ready') return
    FileDownload.offer(state.name, await documents.save())
  }

  return (
    <button type="button" className="button" disabled={state.phase !== 'ready'} onClick={save}>
      Save
    </button>
  )
}
