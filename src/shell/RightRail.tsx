import { useApplication } from '@app/ApplicationContext.ts'
import { useCommentsState } from '@comments/useCommentsState.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import type { RightPanel } from '@shell/PanelStore.ts'
import { usePanelState } from '@shell/usePanelState.ts'
import type { IconName } from '@ui/Icon.tsx'
import { IconButton } from '@ui/IconButton.tsx'
import { PageNavigator } from '@viewer/PageNavigator.tsx'
import { ZoomControls } from '@viewer/ZoomControls.tsx'

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
