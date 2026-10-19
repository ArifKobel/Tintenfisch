import { ProtectionForm } from '@protection/ProtectionForm.tsx'
import { OwnerPasswordForm } from '@protection/OwnerPasswordForm.tsx'
import { useProtectionState } from '@protection/useProtectionState.ts'

export function ProtectionDialog() {
  const { dialogOpen, current } = useProtectionState()
  if (!dialogOpen || !current) return null
  return (
    <div className="dialog-backdrop">{current.ownerRights ? <ProtectionForm current={current} /> : <OwnerPasswordForm />}</div>
  )
}
