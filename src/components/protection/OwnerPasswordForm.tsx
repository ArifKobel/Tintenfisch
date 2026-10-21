import { useState, type FormEvent } from 'react'
import { useProtectionStore } from '@/stores/protectionStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Notice } from '@/components/ui/Text.tsx'
import { Dialog, DialogActions } from '@/components/ui/Dialog.tsx'
import { Input } from '@/components/ui/Field.tsx'

export function OwnerPasswordForm() {
  const protection = useProtectionStore()
  const { notice } = protection
  const [password, setPassword] = useState('')

  function submit(event: FormEvent) {
    event.preventDefault()
    void protection.grantOwnerRights(password)
  }

  return (
    <Dialog title="Protection" label="Protection" onSubmit={submit}>
      <p className="text-ink-body">This document is protected. Enter its permissions password to change or remove the protection.</p>
      <Input type="password" aria-label="Permissions password" autoComplete="off" autoFocus value={password} onChange={(event) => setPassword(event.target.value)} />
      {notice && (
        <Notice role="alert">{notice}</Notice>
      )}
      <DialogActions>
        <Button onClick={() => protection.closeDialog()}>
          Cancel
        </Button>
        <Button type="submit" look="primary">
          Unlock
        </Button>
      </DialogActions>
    </Dialog>
  )
}
