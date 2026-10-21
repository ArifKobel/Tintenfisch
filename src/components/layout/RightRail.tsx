import { useApplication } from '@/app/ApplicationContext.ts'
import { useCommentsState } from '@/hooks/useCommentsState.ts'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import type { RightPanel } from '@/stores/PanelStore.ts'
import { usePanelState } from '@/hooks/usePanelState.ts'
import type { IconName } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { PageNavigator } from '@/components/viewer/PageNavigator.tsx'
import { ZoomControls } from '@/components/viewer/ZoomControls.tsx'

export function RightRail() {
  const { panels } = useApplication()
  const { right } = usePanelState()
  const { comments } = useCommentsState()
  const document = useDocumentState()
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
          <IconButton key={panel} icon={icon} label={label} tooltip="left" look={right === panel ? 'toggled' : null} pressed={right === panel} onClick={() => panels.toggleRight(panel)} />
        ))}
      </div>
      <PageNavigator document={document} />
      <ZoomControls document={document} />
    </nav>
  )
}
