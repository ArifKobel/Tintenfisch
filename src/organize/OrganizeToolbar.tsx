import { useState, type ChangeEvent, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { FileDownload } from '@document/FileDownload.ts'
import { useOrganizeState } from '@organize/useOrganizeState.ts'

export function OrganizeToolbar() {
  const { organize } = useApplication()
  const { selection, notice, lockedInsert } = useOrganizeState()
  const [password, setPassword] = useState('')
  const nothingSelected = selection.size === 0

  async function insertFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await organize.insertFile(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  async function extract() {
    const extracted = await organize.extract()
    if (extracted) FileDownload.offer(extracted.name, extracted.bytes)
  }

  function unlock(event: FormEvent) {
    event.preventDefault()
    void organize.unlockInsert(password)
    setPassword('')
  }

  return (
    <div className="organize-toolbar" role="toolbar" aria-label="Organize pages">
      <button type="button" className="button button-small" disabled={nothingSelected} onClick={() => void organize.rotate(-1)}>
        Rotate left
      </button>
      <button type="button" className="button button-small" disabled={nothingSelected} onClick={() => void organize.rotate(1)}>
        Rotate right
      </button>
      <button type="button" className="button button-small" disabled={nothingSelected} onClick={() => void organize.deleteSelected()}>
        Delete
      </button>
      <button type="button" className="button button-small" disabled={nothingSelected} onClick={() => void extract()}>
        Extract
      </button>
      <button type="button" className="button button-small" onClick={() => void organize.insertBlank()}>
        Blank page
      </button>
      <label className="button button-small">
        Insert file
        <input type="file" accept="application/pdf" onChange={insertFile} hidden />
      </label>
      {lockedInsert && (
        <form className="organize-password" onSubmit={unlock}>
          <span>{lockedInsert.wrongPassword ? 'Wrong password' : `${lockedInsert.name} is protected`}</span>
          <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} aria-label="Password" autoFocus />
          <button type="submit" className="button button-small">
            Insert
          </button>
          <button type="button" className="button button-small" onClick={() => organize.cancelInsert()}>
            Cancel
          </button>
        </form>
      )}
      {notice && (
        <p className="organize-notice" role="status">
          {notice}
        </p>
      )}
    </div>
  )
}
