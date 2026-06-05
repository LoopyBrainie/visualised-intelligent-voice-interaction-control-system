import { type } from 'arktype';

// 每个字面量对应 Rust AppError 的一个 variant —— 名称必须精确匹配 PascalCase
export const appErrorSchema = type({
  name: "'UnrecognizedCommand' | 'VlmTimeout' | 'VlmParseError' | 'VlmLowConfidence' | 'InvalidValue' | 'DeviceNotFound' | 'LockError' | 'AudioDevice' | 'CameraError' | 'NetworkError' | 'PythonEnv' | 'DaemonError' | 'GestureError' | 'InternalError'",
  message: 'string',
  'input?': 'string',
  'detail?': 'string',
  'confidence?': 'number',
  'expected?': 'string',
  'got?': 'number',
  'device?': 'string',
});

export type AppError = typeof appErrorSchema.infer;

type AppErrorName = AppError['name'];

const ERROR_MESSAGES: Record<AppErrorName, string> = {
  UnrecognizedCommand: '无法识别语音指令，请重试',
  VlmTimeout: 'AI 服务请求超时，请检查网络连接',
  VlmParseError: 'AI 响应解析失败，请稍后重试',
  VlmLowConfidence: '识别置信度过低，请重新说出指令',
  InvalidValue: '指令参数无效，请重试',
  DeviceNotFound: '目标设备未找到',
  LockError: '系统内部锁定错误，请重启应用',
  AudioDevice: '音频设备错误，请检查麦克风连接',
  NetworkError: '网络连接失败，请检查网络',
  PythonEnv: 'Python 环境错误，请检查安装',
  DaemonError: '后台服务错误，请重启应用',
  GestureError: '手势识别错误，请重试',
  InternalError: '系统内部错误，请重启应用',
};

export function getUserMessage(error: AppError): string {
  // CameraError 不再被通用文案覆盖 —— 直接显示 nokhwa 真实错误消息
  // (例如 "无法打开摄像头: ..."),这样用户能立即看到真实原因
  // (权限被拒/无兼容格式/设备被占用 等)
  if (error.name === 'CameraError') {
    return error.message || '摄像头错误，请检查摄像头连接';
  }
  return ERROR_MESSAGES[error.name] ?? error.message;
}
