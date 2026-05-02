import { Component, Show, Index, createSignal, onMount, onCleanup } from 'solid-js';
import { DeviceIconCard } from '../devices/DeviceIconCard';
import { RoomId } from '../../store/deviceStore';

type DeviceType = 'light' | 'fan' | 'ac' | 'curtain';

interface RoomRowProps {
  roomId: RoomId;
  roomName: string;
  devices: DeviceType[];
  onDeviceClick: (device: DeviceType, roomId: RoomId) => void;
  registerDeviceRef: (device: DeviceType, el: HTMLButtonElement | undefined) => void;
}

export const RoomRow: Component<RoomRowProps> = (props) => {
  const [showGradient, setShowGradient] = createSignal(false);
  let scrollRef!: HTMLDivElement;

  const checkOverflow = () => {
    if (!scrollRef) return;
    const hasOverflow = scrollRef.scrollWidth > scrollRef.clientWidth;
    const notAtEnd = scrollRef.scrollWidth - scrollRef.scrollLeft > scrollRef.clientWidth + 1;
    setShowGradient(hasOverflow && notAtEnd);
  };

  onMount(() => {
    const ro = new ResizeObserver(checkOverflow);
    ro.observe(scrollRef);
    scrollRef.addEventListener('scroll', checkOverflow, { passive: true });
    onCleanup(() => { ro.disconnect(); });
  });

  return (
    <div class="room-row flex flex-col gap-2 py-3">
      {/* 房间标签 */}
      <div class="room-label px-2">
        <span class="text-body-emphasis text-secondary">
          {props.roomName}
        </span>
      </div>

      {/* 设备横向滚动容器 */}
      <div class="room-devices-scroll relative" classList={{ 'is-overflowing': showGradient() }}>
        <Show
          when={props.devices.length > 0}
          fallback={
            <div class="flex items-center justify-center h-20 text-tertiary text-sm">
              暂无设备
            </div>
          }
        >
          <div ref={scrollRef} class="flex gap-3 px-2 overflow-x-auto scroll-smooth show-scrollbar">
            <Index each={props.devices}>
              {(device) => (
                <DeviceIconCard
                  type={device()}
                  roomId={props.roomId}
                  onClick={() => props.onDeviceClick(device(), props.roomId)}
                  ref={(el) => props.registerDeviceRef(device(), el)}
                />
              )}
            </Index>
          </div>
        </Show>

        {/* 滚动渐变遮罩 - 提示右侧有更多内容 */}
        <div class="scroll-gradient pointer-events-none" />
      </div>
    </div>
  );
};