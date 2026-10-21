import { useState, type FormEvent } from 'react'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function PasswordPrompt({ name, wrongPassword }: { name: string; wrongPassword: boolean }) {
  const [password, setPassword] = useState('')

  function unlock(event: FormEvent) {
    event.preventDefault()
    void useDocumentStore.getState().unlock(password)
  }

  return (
    <form className="dialog password-prompt" onSubmit={unlock}>
      <h2 className="dialog-title">{name} is protected</h2>
      <label className="dialog-field">
        Password
        <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} autoFocus />
      </label>
      {wrongPassword && <p className="notice">That password does not open this document.</p>}
      <div className="dialog-actions">
        <button type="submit" className="button button-primary">
          Unlock
        </button>
      </div>
    </form>
  )
}
