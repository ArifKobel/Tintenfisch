import type { SignatureInfo } from '@/types/protocol.ts'
import { useDigitalSignatureState } from '@/hooks/useDigitalSignatureState.ts'
import { Icon } from '@/components/ui/Icon.tsx'

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
    <ul className="message-bar" aria-label="Signatures">
      {reports.map((report) => (
        <li key={report.fieldId} className={`signature-status signature-status-${report.status}`} title={report.reason ? `Reason: ${report.reason}` : undefined}>
          <Icon name="badge-verified" />
          Signed by {report.signer ?? report.fieldName}: {verdict(report)}
        </li>
      ))}
    </ul>
  )
}
