import { useState, type FormEvent } from 'react'
import { useProtectionStore } from '@/stores/protectionStore.ts'

export function OwnerPasswordForm() {
  const protection = useProtectionStore()
  const { notice } = protection
  const [password, setPassword] = useState('')

  function submit(event: FormEvent) {
    event.preventDefault()
    void protection.grantOwnerRights(password)
  }

  return (
    <form className="dialog link-dialog" role="dialog" aria-modal="true" aria-label="Protection" onSubmit={submit}>
      <h2 className="dialog-title">Protection</h2>
      <p className="dialog-hint">This document is protected. Enter its permissions password to change or remove the protection.</p>
      <input type="password" aria-label="Permissions password" autoComplete="off" autoFocus value={password} onChange={(event) => setPassword(event.target.value)} />
      {notice && (
        <p className="notice" role="alert">
          {notice}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" className="button" onClick={() => protection.closeDialog()}>
          Cancel
        </button>
        <button type="submit" className="button button-primary">
          Unlock
        </button>
      </div>
    </form>
  )
}
