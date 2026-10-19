import { ImageControls } from '@images/ImageControls.tsx'
import { LinkControls } from '@navigation/LinkControls.tsx'
import { RedactionControl } from '@redaction/RedactionControl.tsx'
import { ToolOptions } from '@tools/ToolOptions.tsx'

export function ContextBar() {
  return (
    <div className="context-bar">
      <ToolOptions />
      <ImageControls />
      <LinkControls />
      <RedactionControl />
    </div>
  )
}
