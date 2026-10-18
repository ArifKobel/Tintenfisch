import { useState, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useProtectionState } from '@protection/useProtectionState.ts'

export function OwnerPasswordForm() {
  const { protection } = useApplication()
  const { notice } = useProtectionState()
  const [password, setPassword] = useState('')

  function submit(event: FormEvent) {
    event.preventDefault()
    void protection.grantOwnerRights(password)
  }

  return (
    <form className="link-dialog" role="dialog" aria-modal="true" aria-label="Protection" onSubmit={submit}>
      <h2 className="link-title">Protection</h2>
      <p className="protection-hint">This document is protected. Enter its permissions password to change or remove the protection.</p>
      <input type="password" aria-label="Permissions password" autoComplete="off" autoFocus value={password} onChange={(event) => setPassword(event.target.value)} />
      {notice && (
        <p className="edit-notice" role="alert">
          {notice}
        </p>
      )}
      <div className="link-actions">
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
