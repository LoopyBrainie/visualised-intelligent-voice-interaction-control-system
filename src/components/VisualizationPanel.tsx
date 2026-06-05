import { createSignal, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { useFloating } from 'solid-floating-ui';
import { offset, flip, shift, autoUpdate } from '@floating-ui/dom';
import { Light } from './devices/Light';
import { Fan } from './devices/Fan';
import { AirCondition } from './devices/AirCondition';
import { Curtain } from './devices/Curtain';
import { RoomRow } from './rooms/RoomRow';
import { RoomId, deviceState } from '../store/deviceStore';
import { LiquidGlass } from './ui/LiquidGlass';

type DeviceType = 'light' | 'fan' | 'ac' | 'curtain';

interface Room {
  id: RoomId;
  name: string;
  devices: DeviceType[];
}

const roomsConfig: Room[] = [
  { id: 'living_room', name: '客厅', devices: ['light', 'ac', 'fan', 'curtain'] },
  { id: 'bedroom', name: '卧室', devices: ['light', 'ac'] },
];

// Popover 宽度按设备类型分 — 内容密度差异大,统一宽度会让 Curtain 显得
// 空旷、AC 显得拥挤。slider/stepper/speed-buttons 各取所需。
const popoverWidths: Record<DeviceType, string> = {
  light: 'min-w-[240px] max-w-[300px]',   // slider wants room
  fan: 'min-w-[200px] max-w-[260px]',     // 3 speed buttons
  ac: 'min-w-[240px] max-w-[300px]',      // stepper + temperature
  curtain: 'min-w-[160px] max-w-[200px]', // compact, single row
};

interface ActiveDeviceInfo {
  type: DeviceType;
  roomId: RoomId;
}

export function VisualizationPanel() {
  const [activeDevice, setActiveDevice] = createSignal<ActiveDeviceInfo | null>(null);
  const [closing, setClosing] = createSignal(false);

  let floatingRef: HTMLDivElement | undefined;
  const cardRefs = new Map<string, HTMLButtonElement>();

  const position = useFloating(
    () => activeDevice() ? { getBoundingClientRect: () => getDeviceRect() } : null,
    () => floatingRef,
    {
      whileElementsMounted: autoUpdate,
      middleware: [offset(10), flip(), shift()],
    }
  );

  const getDeviceRect = () => {
    const device = activeDevice();
    if (!device) return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };
    const key = `${device.roomId}_${device.type}`;
    const cardEl = cardRefs.get(key);
    if (!cardEl) return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };
    return cardEl.getBoundingClientRect();
  };

  const closePanel = () => {
    setClosing(true);
  };

  const onPanelAnimationEnd = (e: AnimationEvent) => {
    if (e.animationName === 'panel-exit') {
      setActiveDevice(null);
      setClosing(false);
    }
  };

  const handleDeviceClick = async (device: DeviceType, roomId: RoomId) => {
    const current = activeDevice();
    if (current && current.type === device && current.roomId === roomId) {
      closePanel();
      return;
    }
    if (current) {
      closePanel();
      await new Promise<void>((r) => requestAnimationFrame(() => r()));
    }
    setActiveDevice({ type: device, roomId });
    setClosing(false);
  };

  const registerCardRef = (device: DeviceType, roomId: RoomId, el: HTMLButtonElement | undefined) => {
    if (el) cardRefs.set(`${roomId}_${device}`, el);
  };

  const activeRoomState = () => {
    const dev = activeDevice();
    if (!dev) return null;
    return deviceState()[dev.roomId];
  };

  return (
    <div>
      <div class="rooms-grid">
        {roomsConfig.map((room) => (
          <RoomRow
            roomId={room.id}
            roomName={room.name}
            devices={room.devices}
            onDeviceClick={handleDeviceClick}
            registerDeviceRef={(device, el) => registerCardRef(device, room.id, el)}
          />
        ))}
      </div>

      {/* Floating device control panel */}
      <Show when={activeDevice()}>
        {(device) => <Portal>
          <div
            ref={(el) => { floatingRef = el; }}
            class={`z-floating ${closing() ? 'panel-exit' : 'panel-enter'}`}
            style={{
              position: 'fixed',
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
              'transform-origin': 'top left',
            }}
            onanimationend={onPanelAnimationEnd}
          >
          <LiquidGlass
            radius={16}
            background="rgba(235, 235, 240, 0.85)"
            blur={20}
            edgeBlur={5}
            displacementScale={35}
            contrast={1.2}
            class={`p-4 floating-panel ${popoverWidths[device().type]}`}
          >
            <Show when={device().type === 'light'}>
              <Light roomId={device().roomId} roomState={activeRoomState()!} />
            </Show>
            <Show when={device().type === 'fan'}>
              <Fan roomId={device().roomId} roomState={activeRoomState()!} />
            </Show>
            <Show when={device().type === 'ac'}>
              <AirCondition roomId={device().roomId} roomState={activeRoomState()!} />
            </Show>
            <Show when={device().type === 'curtain'}>
              <Curtain roomId={device().roomId} roomState={activeRoomState()!} />
            </Show>
          </LiquidGlass>
          </div>
        </Portal>}
      </Show>
    </div>
  );
}
