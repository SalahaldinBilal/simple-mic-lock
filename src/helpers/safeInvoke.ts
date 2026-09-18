import { DevicesSnapshot } from "@core/types";
import { invoke, InvokeOptions } from "@tauri-apps/api/core";

export async function safeInvoke<T extends keyof Commands>(cmd: T, ...[args, options]: SafeInvokeArgs<Commands[T]["parameters"]>): Promise<Commands[T]["return"]> {
  return await invoke(cmd, args, options);
}

type SafeInvokeArgs<T> =
  undefined extends T ? [args?: T, options?: InvokeOptions] : [args: T, options?: InvokeOptions];

type Command<P extends object | undefined = undefined, R = undefined> = { parameters: P, return: R }

type Commands = {
  'list_devices': Command<undefined, DevicesSnapshot>,
  'set_target': Command<{ key: string, percent: number }>,
  'set_locked': Command<{ key: string, locked: boolean }>,
  'set_nickname': Command<{ key: string, nickname: string | null }>,
  'claim_device': Command<{ endpoint: string, key: string | null }>,
  'forget_device': Command<{ key: string }>,
  'get_autostart': Command<undefined, boolean>,
  'set_autostart': Command<{ enabled: boolean }>,
  'get_show_tray': Command<undefined, boolean>,
  'set_show_tray': Command<{ enabled: boolean }>,
  'hide_window': Command,
  'quit': Command,
};
