import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { Icon } from '@/components/ui/Icon.tsx'
import { MenuButton, MenuItem } from '@/components/ui/Menu.tsx'
import { useDigitalSignatureStore } from '@/stores/digitalSignatureStore.ts'
import { useSignatureStore } from '@/stores/signatureStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function SignMenuButton() {
  const workspace = useWorkspaceStore()
  const signing = useSignatureStore()
  const digitalSignatures = useDigitalSignatureStore()
  const { tool } = workspace
  const active = tool === 'sign'
  return (
    <MenuButton icon="pen-brush" label="Sign" tooltip="right" look={active ? 'selected' : null} opens="right">
      {(close) => (
        <>
          <MenuItem
            chosen={active}
            onClick={() => {
              workspace.choose('sign')
              close()
            }}
          >
            <Icon name={TOOL_CATALOG.sign.icon} />
            {TOOL_CATALOG.sign.label}
          </MenuItem>
          <MenuItem
            onClick={() => {
              signing.openPad()
              close()
            }}
          >
            <Icon name="draw" />
            Create signature
          </MenuItem>
          <MenuItem
            onClick={() => {
              digitalSignatures.openDialog()
              close()
            }}
          >
            <Icon name="badge-verified" />
            Sign with certificate
          </MenuItem>
        </>
      )}
    </MenuButton>
  )
}
