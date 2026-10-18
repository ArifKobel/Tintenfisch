import { LinkForm } from '@navigation/LinkForm.tsx'
import { useLinksState } from '@navigation/useLinksState.ts'

export function LinkDialog() {
  const { draft } = useLinksState()
  if (!draft) return null
  return <LinkForm key={`${draft.pageId}:${draft.linkId ?? 'new'}`} draft={draft} />
}
