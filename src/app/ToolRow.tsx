import { EditingNotice } from '@editing/EditingNotice.tsx'
import { FormatBar } from '@editing/FormatBar.tsx'
import { FormsNotice } from '@forms/FormsNotice.tsx'
import { ImageControls } from '@images/ImageControls.tsx'
import { LinkControls } from '@navigation/LinkControls.tsx'
import { RedactionControl } from '@redaction/RedactionControl.tsx'
import { SignatureStatusList } from '@signing/SignatureStatusList.tsx'
import { ToolPicker } from '@tools/ToolPicker.tsx'

export function ToolRow() {
  return (
    <div className="tool-row">
      <ToolPicker />
      <FormatBar />
      <ImageControls />
      <LinkControls />
      <EditingNotice />
      <FormsNotice />
      <RedactionControl />
      <SignatureStatusList />
    </div>
  )
}
