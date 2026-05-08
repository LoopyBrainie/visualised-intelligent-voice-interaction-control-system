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
import { useSnapshot } from './ui/SnapshotContext';

type DeviceType = 'light' | 'fan' | 'ac' | 'curtain';

interface Room {
  id: RoomId;
  name: string;
  devices: DeviceType[];
}

// 房间-设备布局配置
const roomsConfig: Room[] = [
  {
    id: 'living_room',
    name: '客厅',
    devices: ['light', 'ac', 'fan', 'curtain'],
  },
  {
    id: 'bedroom',
    name: '卧室',
    devices: ['light', 'ac'],
  },
];

// 激活设备信息（设备类型 + 房间）
interface ActiveDeviceInfo {
  type: DeviceType;
  roomId: RoomId;
}

export function VisualizationPanel() {
  const [activeDevice, setActiveDevice] = createSignal<ActiveDeviceInfo | null>(null);
  const { notifyPanelOpen, notifyPanelClose } = useSnapshot();

  // Floating UI references - 使用 roomId_type 作为 key
  let floatingRef: HTMLDivElement | undefined;
  let cardRefs: Map<string, HTMLButtonElement> = new Map();

  const position = useFloating(
    () => activeDevice() ? { getBoundingClientRect: () => getDeviceRect() } : null,
    () => floatingRef,
    {
      whileElementsMounted: autoUpdate,
      middleware: [offset(10), flip(), shift()],
    }
  );

  // 获取当前激活设备卡片的 bounding rect
  const getDeviceRect = () => {
    const device = activeDevice();
    if (!device) return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };

    const key = `${device.roomId}_${device.type}`;
    const cardEl = cardRefs.get(key);
    if (!cardEl) return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };

    const rect = cardEl.getBoundingClientRect();
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      top: rect.top,
      left: rect.left,
      right: rect.right,
      bottom: rect.bottom,
    };
  };

  const handleDeviceClick = async (device: DeviceType, roomId: RoomId) => {
    const current = activeDevice();
    if (current && current.type === device && current.roomId === roomId) {
      setActiveDevice(null);
      notifyPanelClose();
    } else {
      // Capture snapshot BEFORE panel renders (prevents mirror-in-mirror)
      await notifyPanelOpen();
      setActiveDevice({ type: device, roomId });
    }
  };

  // 注册卡片 ref，使用 roomId_type 作为 key
  const registerCardRef = (device: DeviceType, roomId: RoomId, el: HTMLButtonElement | undefined) => {
    if (el) {
      cardRefs.set(`${roomId}_${device}`, el);
    }
  };

  // 获取当前激活设备的房间状态
  const activeRoomState = () => {
    const dev = activeDevice();
    if (!dev) return null;
    return deviceState()[dev.roomId];
  };

  return (
    <div class="h-full flex flex-col">
      {/* 设备选择器标签 */}
      <div class="flex items-center gap-2 mb-4">
        <div class="w-2 h-2 rounded-full bg-accent animate-pulse" />
        <h3 class="text-sm font-semibold text-primary tracking-tight">
          设备控制中心
        </h3>
      </div>

      {/* 房间-设备列表 */}
      <div class="flex-1 overflow-y-auto">
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

      {/* 浮动设备控制面板 - 使用 Portal 渲染到 body 层级 */}
      <Show when={activeDevice()}>
        {(device) => <Portal>
          <LiquidGlass
            ref={(el) => { floatingRef = el; }}
            radius={16}
            blur={5}
            background="rgba(255, 255, 255, 0.8)"
            class="p-4 min-w-[200px] max-w-[280px] z-modal absolute shadow-apple"
            style={{
              position: 'fixed',
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
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
        </Portal>}
      </Show>
    </div>
  );
}