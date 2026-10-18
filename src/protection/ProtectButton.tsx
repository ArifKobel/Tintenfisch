import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { useProtectionState } from '@protection/useProtectionState.ts'

export function ProtectButton() {
  const { protection } = useApplication()
  const { pending } = useProtectionState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <button type="button" className="button" title={pending ?? undefined} onClick={() => void protection.openDialog()}>
      Protect
    </button>
  )
}
