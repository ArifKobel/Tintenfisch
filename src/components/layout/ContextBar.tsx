import { ImageControls } from '@/components/images/ImageControls.tsx'
import { LinkControls } from '@/components/links/LinkControls.tsx'
import { RedactionControl } from '@/components/redaction/RedactionControl.tsx'
import { ToolOptions } from '@/components/tools/ToolOptions.tsx'

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
