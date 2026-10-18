import { useState, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function PasswordPrompt({ name, wrongPassword }: { name: string; wrongPassword: boolean }) {
  const { documents } = useApplication()
  const [password, setPassword] = useState('')

  function unlock(event: FormEvent) {
    event.preventDefault()
    void documents.unlock(password)
  }

  return (
    <form className="password-prompt" onSubmit={unlock}>
      <h2>{name} is protected</h2>
      <label>
        Password
        <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} autoFocus />
      </label>
      {wrongPassword && <p className="password-wrong">That password does not open this document.</p>}
      <button type="submit" className="button button-primary">
        Unlock
      </button>
    </form>
  )
}
