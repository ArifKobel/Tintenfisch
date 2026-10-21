import { ProtectionForm } from '@/components/protection/ProtectionForm.tsx'
import { OwnerPasswordForm } from '@/components/protection/OwnerPasswordForm.tsx'
import { useProtectionStore } from '@/stores/protectionStore.ts'

export function ProtectionDialog() {
  const { dialogOpen, current } = useProtectionStore()
  if (!dialogOpen || !current) return null
  return (
    <div className="dialog-backdrop">{current.ownerRights ? <ProtectionForm current={current} /> : <OwnerPasswordForm />}</div>
  )
}
