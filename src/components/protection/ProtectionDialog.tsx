import { ProtectionForm } from '@/components/protection/ProtectionForm.tsx'
import { OwnerPasswordForm } from '@/components/protection/OwnerPasswordForm.tsx'
import { useProtectionState } from '@/hooks/useProtectionState.ts'

export function ProtectionDialog() {
  const { dialogOpen, current } = useProtectionState()
  if (!dialogOpen || !current) return null
  return (
    <div className="dialog-backdrop">{current.ownerRights ? <ProtectionForm current={current} /> : <OwnerPasswordForm />}</div>
  )
}
