import { Icon } from '@/components/ui/Icon.tsx'
import { MenuButton, MenuItem } from '@/components/ui/Menu.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useFontStore } from '@/stores/fontStore.ts'
import { usePrintStore } from '@/stores/printStore.ts'
import { useProtectionStore } from '@/stores/protectionStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function DocumentMenu() {
  const phase = useDocumentStore((state) => state.document.phase)
  if (phase !== 'ready') return null
  const entries = [
    { label: 'Print', icon: 'print', run: () => usePrintStore.getState().openDialog() },
    { label: 'Protect with password', icon: 'lock', run: () => void useProtectionStore.getState().openDialog() },
    { label: 'Organize pages', icon: 'view-grid', run: () => useWorkspaceStore.getState().showRight('pages') },
    { label: 'Font downloads', icon: 'download', run: () => useFontStore.getState().review() },
  ] as const
  return (
    <MenuButton icon="more" label="More options" opens="below">
      {(close) =>
        entries.map(({ label, icon, run }) => (
          <MenuItem
            key={label}
            onClick={() => {
              run()
              close()
            }}
          >
            <Icon name={icon} />
            {label}
          </MenuItem>
        ))
      }
    </MenuButton>
  )
}
