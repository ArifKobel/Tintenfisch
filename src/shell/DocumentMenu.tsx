import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { Icon } from '@ui/Icon.tsx'
import { MenuButton } from '@ui/MenuButton.tsx'

export function DocumentMenu() {
  const { printing, protection, panels, fontConsent } = useApplication()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  const entries = [
    { label: 'Print', icon: 'print', run: () => printing.openDialog() },
    { label: 'Protect with password', icon: 'lock', run: () => void protection.openDialog() },
    { label: 'Organize pages', icon: 'view-grid', run: () => panels.showRight('pages') },
    { label: 'Font downloads', icon: 'download', run: () => fontConsent.review() },
  ] as const
  return (
    <MenuButton icon="more" label="More options" opens="below">
      {(close) =>
        entries.map(({ label, icon, run }) => (
          <button
            key={label}
            type="button"
            role="menuitem"
            className="menu-item"
            onClick={() => {
              run()
              close()
            }}
          >
            <Icon name={icon} />
            {label}
          </button>
        ))
      }
    </MenuButton>
  )
}
