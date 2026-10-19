import { useState, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { Permissions, ProtectionState } from '@engine/protocol.ts'
import { useProtectionState } from '@protection/useProtectionState.ts'

const PERMISSIONS: { key: keyof Permissions; label: string }[] = [
  { key: 'print', label: 'Printing' },
  { key: 'printFaithful', label: 'Printing in high quality' },
  { key: 'copy', label: 'Copying text and images' },
  { key: 'modify', label: 'Changing the document' },
  { key: 'annotate', label: 'Commenting' },
  { key: 'fillForms', label: 'Filling in forms and signing' },
  { key: 'assemble', label: 'Inserting, deleting and rotating pages' },
]

const EVERYTHING: Permissions = { print: true, printFaithful: true, modify: true, copy: true, annotate: true, fillForms: true, assemble: true }

export function ProtectionForm({ current }: { current: ProtectionState }) {
  const { protection } = useApplication()
  const { notice } = useProtectionState()
  const restricted = Object.values(current.permissions).some((granted) => !granted)
  const [openWithPassword, setOpenWithPassword] = useState(false)
  const [userPassword, setUserPassword] = useState('')
  const [userRepeated, setUserRepeated] = useState('')
  const [restrict, setRestrict] = useState(current.encrypted && restricted)
  const [ownerPassword, setOwnerPassword] = useState('')
  const [ownerRepeated, setOwnerRepeated] = useState('')
  const [permissions, setPermissions] = useState<Permissions>(current.permissions)

  const problem = (() => {
    if (!openWithPassword && !restrict) return 'Choose a password to open the document, one to restrict it, or both'
    if (openWithPassword && userPassword === '') return 'Enter the password to open the document'
    if (openWithPassword && userPassword !== userRepeated) return 'The passwords to open the document differ'
    if (restrict && ownerPassword === '') return 'Enter the permissions password'
    if (restrict && ownerPassword !== ownerRepeated) return 'The permissions passwords differ'
    if (openWithPassword && restrict && userPassword === ownerPassword) return 'The two passwords must differ'
    return null
  })()

  function submit(event: FormEvent) {
    event.preventDefault()
    if (problem) return
    void protection.protect(openWithPassword ? userPassword : '', restrict ? ownerPassword : '', restrict ? permissions : EVERYTHING)
  }

  return (
    <form className="dialog protection-dialog" role="dialog" aria-modal="true" aria-label="Protection" onSubmit={submit}>
      <h2 className="dialog-title">Protection</h2>
      <p className="dialog-hint">{current.encrypted ? 'This document is protected with a password.' : 'This document is not protected.'} New protection uses AES-256.</p>
      <label className="dialog-choice">
        <input type="checkbox" checked={openWithPassword} onChange={(event) => setOpenWithPassword(event.target.checked)} />
        Require a password to open the document
      </label>
      {openWithPassword && (
        <div className="protection-passwords">
          <input type="password" aria-label="Password to open" placeholder="Password" autoComplete="new-password" value={userPassword} onChange={(event) => setUserPassword(event.target.value)} />
          <input type="password" aria-label="Repeat the password to open" placeholder="Repeat" autoComplete="new-password" value={userRepeated} onChange={(event) => setUserRepeated(event.target.value)} />
          {!restrict && <p className="hint">Anyone with this password can also change or remove the protection.</p>}
        </div>
      )}
      <label className="dialog-choice">
        <input type="checkbox" checked={restrict} onChange={(event) => setRestrict(event.target.checked)} />
        Restrict what others may do
      </label>
      {restrict && (
        <>
          <div className="protection-passwords">
            <input type="password" aria-label="Permissions password" placeholder="Permissions password" autoComplete="new-password" value={ownerPassword} onChange={(event) => setOwnerPassword(event.target.value)} />
            <input type="password" aria-label="Repeat the permissions password" placeholder="Repeat" autoComplete="new-password" value={ownerRepeated} onChange={(event) => setOwnerRepeated(event.target.value)} />
          </div>
          <fieldset className="protection-permissions">
            <legend>Allow</legend>
            {PERMISSIONS.map(({ key, label }) => (
              <label key={key} className="dialog-choice">
                <input type="checkbox" checked={permissions[key]} onChange={(event) => setPermissions({ ...permissions, [key]: event.target.checked })} />
                {label}
              </label>
            ))}
          </fieldset>
        </>
      )}
      {(problem || notice) && (
        <p className="dialog-hint" role="status">
          {notice ?? problem}
        </p>
      )}
      <div className="dialog-actions">
        {current.encrypted && (
          <button type="button" className="button" onClick={() => void protection.unprotect()}>
            Remove protection
          </button>
        )}
        <button type="button" className="button" onClick={() => protection.closeDialog()}>
          Cancel
        </button>
        <button type="submit" className="button button-primary" disabled={problem !== null}>
          Apply
        </button>
      </div>
    </form>
  )
}
