import { useDocumentState } from '@document/useDocumentState.ts'
import type { DocumentState } from '@document/DocumentStore.ts'

function describe(state: DocumentState): string {
  switch (state.phase) {
    case 'empty':
      return 'Open a PDF to start.'
    case 'loading':
      return `Opening ${state.name}`
    case 'locked':
      return `${state.name}: password required`
    case 'ready':
      return [
        state.name,
        `PDF ${state.version}`,
        `${state.pageCount.toLocaleString()} ${state.pageCount === 1 ? 'page' : 'pages'}`,
        `${state.objectCount.toLocaleString()} objects`,
        ...(state.repaired ? ['repaired'] : []),
      ].join(' · ')
    case 'failed':
      return `${state.name}: ${state.message}`
  }
}

export function DocumentStatus() {
  const state = useDocumentState()
  return (
    <span className={`status status-${state.phase}`} role="status">
      {describe(state)}
    </span>
  )
}
