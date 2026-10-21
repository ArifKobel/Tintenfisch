import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { Icon } from '@/components/ui/Icon.tsx'
import { MenuButton } from '@/components/ui/MenuButton.tsx'
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
          <button
            type="button"
            role="menuitem"
            className={`menu-item${active ? ' menu-item-chosen' : ''}`}
            onClick={() => {
              workspace.choose('sign')
              close()
            }}
          >
            <Icon name={TOOL_CATALOG.sign.icon} />
            {TOOL_CATALOG.sign.label}
          </button>
          <button
            type="button"
            role="menuitem"
            className="menu-item"
            onClick={() => {
              signing.openPad()
              close()
            }}
          >
            <Icon name="draw" />
            Create signature
          </button>
          <button
            type="button"
            role="menuitem"
            className="menu-item"
            onClick={() => {
              digitalSignatures.openDialog()
              close()
            }}
          >
            <Icon name="badge-verified" />
            Sign with certificate
          </button>
        </>
      )}
    </MenuButton>
  )
}
