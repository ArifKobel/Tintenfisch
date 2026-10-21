import { LinkForm } from '@/components/links/LinkForm.tsx'
import { useLinksStore } from '@/stores/linksStore.ts'

export function LinkDialog() {
  const { draft } = useLinksStore()
  if (!draft) return null
  return <LinkForm key={`${draft.pageId}:${draft.linkId ?? 'new'}`} draft={draft} />
}
