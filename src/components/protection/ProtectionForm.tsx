import { useState, type FormEvent } from 'react'
import type { Permissions, ProtectionState } from '@/types/protocol.ts'
import { useProtectionStore } from '@/stores/protectionStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Hint } from '@/components/ui/Text.tsx'
import { Dialog, DialogActions, DialogChoice } from '@/components/ui/Dialog.tsx'
import { Input } from '@/components/ui/Field.tsx'

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
  const protection = useProtectionStore()
  const { notice } = protection
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
    <Dialog title="Protection" label="Protection" className="max-w-[440px]" onSubmit={submit}>
      <p className="text-ink-body">{current.encrypted ? 'This document is protected with a password.' : 'This document is not protected.'} New protection uses AES-256.</p>
      <DialogChoice>
        <input type="checkbox" checked={openWithPassword} onChange={(event) => setOpenWithPassword(event.target.checked)} />
        Require a password to open the document
      </DialogChoice>
      {openWithPassword && (
        <div className="ml-[22px] grid gap-2">
          <Input type="password" aria-label="Password to open" placeholder="Password" autoComplete="new-password" value={userPassword} onChange={(event) => setUserPassword(event.target.value)} />
          <Input type="password" aria-label="Repeat the password to open" placeholder="Repeat" autoComplete="new-password" value={userRepeated} onChange={(event) => setUserRepeated(event.target.value)} />
          {!restrict && <Hint>Anyone with this password can also change or remove the protection.</Hint>}
        </div>
      )}
      <DialogChoice>
        <input type="checkbox" checked={restrict} onChange={(event) => setRestrict(event.target.checked)} />
        Restrict what others may do
      </DialogChoice>
      {restrict && (
        <>
          <div className="ml-[22px] grid gap-2">
            <Input type="password" aria-label="Permissions password" placeholder="Permissions password" autoComplete="new-password" value={ownerPassword} onChange={(event) => setOwnerPassword(event.target.value)} />
            <Input type="password" aria-label="Repeat the permissions password" placeholder="Repeat" autoComplete="new-password" value={ownerRepeated} onChange={(event) => setOwnerRepeated(event.target.value)} />
          </div>
          <fieldset className="m-0 ml-[22px] grid gap-1.5 rounded border border-separator px-3 py-2.5">
            <legend>Allow</legend>
            {PERMISSIONS.map(({ key, label }) => (
              <DialogChoice key={key}>
                <input type="checkbox" checked={permissions[key]} onChange={(event) => setPermissions({ ...permissions, [key]: event.target.checked })} />
                {label}
              </DialogChoice>
            ))}
          </fieldset>
        </>
      )}
      {(problem || notice) && (
        <p className="text-ink-body" role="status">
          {notice ?? problem}
        </p>
      )}
      <DialogActions>
        {current.encrypted && (
          <Button onClick={() => void protection.unprotect()}>
            Remove protection
          </Button>
        )}
        <Button onClick={() => protection.closeDialog()}>
          Cancel
        </Button>
        <Button type="submit" look="primary" disabled={problem !== null}>
          Apply
        </Button>
      </DialogActions>
    </Dialog>
  )
}
