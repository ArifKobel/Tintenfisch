import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'

export function PrintButton() {
  const { printing } = useApplication()
  const document = useDocumentState()
  return (
    <button type="button" className="button" disabled={document.phase !== 'ready'} onClick={() => printing.openDialog()}>
      Print
    </button>
  )
}
