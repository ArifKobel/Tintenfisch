import { useState, type FormEvent } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { FileDownload } from '@/services/FileDownload.ts'
import { useDigitalSignatureStore } from '@/stores/digitalSignatureStore.ts'
import { useFormsStore } from '@/stores/formsStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Notice } from '@/components/ui/Text.tsx'
import { Dialog, DialogActions, DialogField } from '@/components/ui/Dialog.tsx'
import { Input, Select } from '@/components/ui/Field.tsx'

export function SigningDialog() {
  const digitalSignatures = useDigitalSignatureStore()
  const { dialogOpen, signing, notice, reports } = digitalSignatures
  const { fields } = useFormsStore()
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
    await FileDownload.offer(result.name, result.bytes)
    setPassword('')
  }

  return (
    <Dialog title="Sign with certificate" label="Sign with certificate" onSubmit={(event) => void submit(event)}>
        {unsigned.length === 0 ? (
          <p className="text-ink-body">Place a signature on the page first; signing fills one of the signature fields.</p>
        ) : (
          <>
            <DialogField label="Signature field">
              <Select value={chosen} onChange={(event) => setFieldId(event.target.value)}>
                {unsigned.map((field) => (
                  <option key={field.id} value={field.id}>
                    {field.name}
                  </option>
                ))}
              </Select>
            </DialogField>
            <DialogField label="Certificate file (.p12, .pfx)">
              <Input type="file" accept=".p12,.pfx,application/x-pkcs12" onChange={(event) => setCertificate(event.target.files?.[0] ?? null)} />
            </DialogField>
            <DialogField label="Password">
              <Input type="password" autoComplete="off" value={password} onChange={(event) => setPassword(event.target.value)} />
            </DialogField>
            <DialogField label="Reason">
              <Input value={reason} onChange={(event) => setReason(event.target.value)} />
            </DialogField>
            <DialogField label="Location">
              <Input value={location} onChange={(event) => setLocation(event.target.value)} />
            </DialogField>
            <p className="text-ink-body">Signing saves every change so far and {isTauri() ? 'asks where to save' : 'downloads'} the signed file.</p>
          </>
        )}
        {notice && (
          <Notice>{notice}</Notice>
        )}
        <DialogActions>
          <Button onClick={() => digitalSignatures.closeDialog()} disabled={signing}>
            Cancel
          </Button>
          <Button type="submit" look="primary" disabled={signing || !certificate || !chosen}>
            {signing ? 'Signing' : 'Sign'}
          </Button>
        </DialogActions>
    </Dialog>
  )
}
