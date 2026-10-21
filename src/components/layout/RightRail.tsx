import type { IconName } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { PageNavigator } from '@/components/viewer/PageNavigator.tsx'
import { ZoomControls } from '@/components/viewer/ZoomControls.tsx'
import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore, type RightPanel } from '@/stores/workspaceStore.ts'

export function RightRail() {
  const { right, toggleRight } = useWorkspaceStore()
  const comments = useCommentsStore((state) => state.comments)
  const document = useDocumentStore((state) => state.document)
  if (document.phase !== 'ready') return null
  const toggles: { panel: RightPanel; icon: IconName; label: string }[] = [
    { panel: 'comments', icon: 'comment', label: comments.length ? `Comments (${comments.length})` : 'Comments' },
    { panel: 'pages', icon: 'view-grid', label: 'Pages' },
    { panel: 'bookmarks', icon: 'bookmark', label: 'Bookmarks' },
  ]
  return (
    <nav className="right-rail" aria-label="Panels and navigation">
      <div className="rail-group">
        {toggles.map(({ panel, icon, label }) => (
          <IconButton key={panel} icon={icon} label={label} tooltip="left" look={right === panel ? 'toggled' : null} pressed={right === panel} onClick={() => toggleRight(panel)} />
        ))}
      </div>
      <PageNavigator document={document} />
      <ZoomControls document={document} />
    </nav>
  )
}
