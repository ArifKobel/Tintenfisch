import type { FieldProperties, FormField } from '@/types/protocol.ts'
import { SidePanel } from '@/components/ui/SidePanel.tsx'
import { useFormsStore } from '@/stores/formsStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Input, TextArea } from '@/components/ui/Field.tsx'

function propertiesOf(field: FormField): FieldProperties {
  return {
    name: field.partialName,
    required: field.required,
    readOnly: field.readOnly,
    multiline: field.multiline,
    options: field.options.map((option) => option.label),
  }
}

export function FieldPanel() {
  const forms = useFormsStore()
  const { fields, selected } = forms
  const { tool } = useWorkspaceStore()
  const field = fields.find((candidate) => candidate.id === selected)
  if (tool !== 'form' || !field) return null
  const current = propertiesOf(field)
  const change = (properties: Partial<FieldProperties>) => void forms.setProperties(field.id, { ...current, ...properties })
  const choice = field.kind === 'combo' || field.kind === 'list'
  return (
    <SidePanel title="Field properties" side="right" onClose={() => forms.select(null)}>
      <label className="flex flex-col gap-1">
        Name
        <Input key={field.partialName} defaultValue={field.partialName} onBlur={(event) => event.target.value !== field.partialName && change({ name: event.target.value })} />
      </label>
      <label className="flex items-center gap-2">
        <input type="checkbox" checked={field.required} onChange={(event) => change({ required: event.target.checked })} />
        Required
      </label>
      <label className="flex items-center gap-2">
        <input type="checkbox" checked={field.readOnly} onChange={(event) => change({ readOnly: event.target.checked })} />
        Read only
      </label>
      {field.kind === 'text' && (
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={field.multiline} onChange={(event) => change({ multiline: event.target.checked })} />
          Several lines
        </label>
      )}
      {choice && (
        <label className="flex flex-col gap-1">
          Entries, one per line
          <TextArea
            key={current.options.join('\n')}
            defaultValue={current.options.join('\n')}
            rows={5}
            onBlur={(event) => change({ options: event.target.value.split('\n').map((line) => line.trim()).filter(Boolean) })}
          />
        </label>
      )}
      <div>
        <Button onClick={() => void forms.deleteSelected()}>
          Delete field
        </Button>
      </div>
    </SidePanel>
  )
}
