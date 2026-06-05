import { Component, Index, Show } from 'solid-js';
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
  return (
    <section class="mb-2xl">
      {/* Room header — Tagline */}
      <h2 class="text-tagline text-primary mb-md px-1">
        {props.roomName}
      </h2>

      {/* Device grid — constrained fluid (A+) */}
      <Show
        when={props.devices.length > 0}
        fallback={
          <p class="text-caption text-tertiary text-center py-xl">
            暂无设备
          </p>
        }
      >
        <div class="device-grid">
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
    </section>
  );
};
