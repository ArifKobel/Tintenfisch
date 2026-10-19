import { useApplication } from '@app/ApplicationContext.ts'
import { TOOL_CATALOG } from '@tools/ToolCatalog.ts'
import { useToolState } from '@tools/useToolState.ts'
import { Icon } from '@ui/Icon.tsx'
import { MenuButton } from '@ui/MenuButton.tsx'

export function SignMenuButton() {
  const { tools, signing, digitalSignatures } = useApplication()
  const { tool } = useToolState()
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
              tools.choose('sign')
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
