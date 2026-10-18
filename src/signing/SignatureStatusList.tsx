import type { SignatureInfo } from '@engine/protocol.ts'
import { useDigitalSignatureState } from '@signing/useDigitalSignatureState.ts'

function verdict(report: SignatureInfo): string {
  switch (report.status) {
    case 'valid':
      return report.changedAfter ? 'valid, changed since' : 'valid'
    case 'broken':
      return `broken: ${report.problem ?? 'unknown reason'}`
    case 'unsupported':
      return `not checked: ${report.problem ?? 'unknown reason'}`
  }
}

export function SignatureStatusList() {
  const { reports } = useDigitalSignatureState()
  if (reports.length === 0) return null
  return (
    <ul className="signature-status" aria-label="Signatures">
      {reports.map((report) => (
        <li key={report.fieldId} className={`signature-status-${report.status}`} title={report.reason ? `Reason: ${report.reason}` : undefined}>
          Signed by {report.signer ?? report.fieldName}: {verdict(report)}
        </li>
      ))}
    </ul>
  )
}
