import { invoke } from '@tauri-apps/api/core';
import { type } from 'arktype';
import { appErrorSchema, type AppError } from './AppError';

export type InvokeResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: AppError };

export async function typedInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<InvokeResult<T>> {
  try {
    const value = await invoke<T>(command, args);
    return { ok: true, value };
  } catch (unknownError) {
    const result = appErrorSchema(unknownError);
    if (result instanceof type.errors) {
      return {
        ok: false,
        error: {
          name: 'InternalError' as const,
          message: String(unknownError),
        },
      };
    }
    return { ok: false, error: result };
  }
}
