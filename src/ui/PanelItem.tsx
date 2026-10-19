import { Icon, type IconName } from '@ui/Icon.tsx'

export function PanelItem({ icon, label, chosen = false, onClick }: { icon: IconName; label: string; chosen?: boolean; onClick: () => void }) {
  return (
    <button type="button" className={`panel-item${chosen ? ' panel-item-chosen' : ''}`} aria-pressed={chosen} onClick={onClick}>
      <Icon name={icon} />
      {label}
    </button>
  )
}
