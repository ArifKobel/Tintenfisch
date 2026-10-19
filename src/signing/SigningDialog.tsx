import { useState, type FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { FileDownload } from '@document/FileDownload.ts'
import { useFormsState } from '@forms/useFormsState.ts'
import { useDigitalSignatureState } from '@signing/useDigitalSignatureState.ts'

export function SigningDialog() {
  const { digitalSignatures } = useApplication()
  const { dialogOpen, signing, notice, reports } = useDigitalSignatureState()
  const { fields } = useFormsState()
  const [fieldId, setFieldId] = useState('')
  const [certificate, setCertificate] = useState<File | null>(null)
  const [password, setPassword] = useState('')
  const [reason, setReason] = useState('')
  const [location, setLocation] = useState('')
  if (!dialogOpen) return null
  const signed = new Set(reports.map((report) => report.fieldId))
  const unsigned = fields.filter((field) => field.kind === 'signature' && !signed.has(field.id))
  const chosen = unsigned.some((field) => field.id === fieldId) ? fieldId : (unsigned[0]?.id ?? '')

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!certificate || !chosen) return
    const bytes = new Uint8Array(await certificate.arrayBuffer())
    const result = await digitalSignatures.sign({ fieldId: chosen, certificate: bytes, password, reason, location })
    if (!result) return
    FileDownload.offer(result.name, result.bytes)
    setPassword('')
  }

  return (
    <div className="dialog-backdrop">
      <form className="dialog signature-pad" role="dialog" aria-modal="true" aria-label="Sign with certificate" onSubmit={(event) => void submit(event)}>
        <h2 className="dialog-title">Sign with certificate</h2>
        {unsigned.length === 0 ? (
          <p className="dialog-hint">Place a signature on the page first; signing fills one of the signature fields.</p>
        ) : (
          <>
            <label className="dialog-field">
              Signature field
              <select value={chosen} onChange={(event) => setFieldId(event.target.value)}>
                {unsigned.map((field) => (
                  <option key={field.id} value={field.id}>
                    {field.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="dialog-field">
              Certificate file (.p12, .pfx)
              <input type="file" accept=".p12,.pfx,application/x-pkcs12" onChange={(event) => setCertificate(event.target.files?.[0] ?? null)} />
            </label>
            <label className="dialog-field">
              Password
              <input type="password" autoComplete="off" value={password} onChange={(event) => setPassword(event.target.value)} />
            </label>
            <label className="dialog-field">
              Reason
              <input value={reason} onChange={(event) => setReason(event.target.value)} />
            </label>
            <label className="dialog-field">
              Location
              <input value={location} onChange={(event) => setLocation(event.target.value)} />
            </label>
            <p className="dialog-hint">Signing saves every change so far and downloads the signed file.</p>
          </>
        )}
        {notice && (
          <p className="notice" role="status">
            {notice}
          </p>
        )}
        <div className="dialog-actions">
          <span className="dialog-spacer" />
          <button type="button" className="button" onClick={() => digitalSignatures.closeDialog()} disabled={signing}>
            Cancel
          </button>
          <button type="submit" className="button button-primary" disabled={signing || !certificate || !chosen}>
            {signing ? 'Signing' : 'Sign'}
          </button>
        </div>
      </form>
    </div>
  )
}
