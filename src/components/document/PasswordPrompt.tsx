import { useState, type FormEvent } from 'react'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { DialogActions, DialogField, DialogPanel } from '@/components/ui/Dialog.tsx'
import { Input } from '@/components/ui/Field.tsx'
import { Notice } from '@/components/ui/Text.tsx'

export function PasswordPrompt({ name, wrongPassword }: { name: string; wrongPassword: boolean }) {
  const [password, setPassword] = useState('')

  function unlock(event: FormEvent) {
    event.preventDefault()
    void useDocumentStore.getState().unlock(password)
  }

  return (
    <DialogPanel title={`${name} is protected`} className="w-[400px]" onSubmit={unlock}>
      <DialogField label="Password">
        <Input type="password" value={password} onChange={(event) => setPassword(event.target.value)} autoFocus />
      </DialogField>
      {wrongPassword && <Notice>That password does not open this document.</Notice>}
      <DialogActions>
        <Button type="submit" look="primary">
          Unlock
        </Button>
      </DialogActions>
    </DialogPanel>
  )
}
