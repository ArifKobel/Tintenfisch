import type { SignatureInfo } from '@/types/protocol.ts'
import { Icon } from '@/components/ui/Icon.tsx'
import { useDigitalSignatureStore } from '@/stores/digitalSignatureStore.ts'

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


const STATUS_COLORS = {
  valid: 'text-positive',
  broken: 'text-negative',
  unsupported: 'text-ink-disabled',
}

export function SignatureStatusList() {
  const { reports } = useDigitalSignatureStore()
  if (reports.length === 0) return null
  return (
    <ul className="m-0 flex list-none flex-wrap gap-x-6 gap-y-1 border-b border-divider bg-surface px-6 py-2.5" aria-label="Signatures">
      {reports.map((report) => (
        <li key={report.fieldId} className="flex items-center gap-2 text-ink-strong" title={report.reason ? `Reason: ${report.reason}` : undefined}>
          <Icon name="badge-verified" className={`size-5 ${STATUS_COLORS[report.status]}`} />
          Signed by {report.signer ?? report.fieldName}: {verdict(report)}
        </li>
      ))}
    </ul>
  )
}
