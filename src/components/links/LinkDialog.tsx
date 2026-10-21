import { LinkForm } from '@/components/links/LinkForm.tsx'
import { useLinksState } from '@/hooks/useLinksState.ts'

export function LinkDialog() {
  const { draft } = useLinksState()
  if (!draft) return null
  return <LinkForm key={`${draft.pageId}:${draft.linkId ?? 'new'}`} draft={draft} />
}
