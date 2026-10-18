export interface Settings {
  deviceName: string;
  port: number;
  rootMode: "all" | "drive" | "folder";
  rootPath: string;
  privateMode: boolean;
  privateFiles: string[];
  showHidden: boolean;
  restrictModify: boolean;
  requireApproval: boolean;
  passwordSet: boolean;
  sessionHours: number;
  https: boolean;
  theme: string;
  portalTheme: string;
  pluginDevelopment: boolean;
  devToken: string;
}
export interface Drive {
  path: string;
  name: string;
  total: number;
  free: number;
  eligible: boolean;
  protection: string;
}
export interface FileItem {
  name: string;
  location: string;
  directory: boolean;
  size: number;
  modified: number;
  mime: string;
  hidden: boolean;
}
export interface Device {
  id: string;
  name: string;
  ip: string;
  status: string;
  remembered: boolean;
  lastSeen: number;
  expires: number;
  plugins: string[];
}
export interface Transfer {
  id: string;
  name: string;
  direction: string;
  size: number;
  transferred: number;
  status: string;
  device: string;
  timestamp: number;
  elapsedMs: number;
  path: string;
}
export interface Plugin {
  uid: string;
  name: string;
  package: string;
  description: string;
  author: string;
  version: string;
  enabled: boolean;
}
export interface Snapshot {
  settings: Settings;
  drives: Drive[];
  running: boolean;
  started: number;
  urls: string[];
  devices: Device[];
  history: Transfer[];
  plugins: Plugin[];
  totals?: { sent: number; received: number };
}
export const defaults: Settings = {
  deviceName: "My desktop",
  port: 6060,
  rootMode: "folder",
  rootPath: "",
  privateMode: false,
  privateFiles: [],
  showHidden: false,
  restrictModify: true,
  requireApproval: true,
  passwordSet: false,
  sessionHours: 24,
  https: false,
  theme: "dark",
  portalTheme: "default",
  pluginDevelopment: false,
  devToken: "",
};
export function size(n: number) {
  if (!n) return "0 B";
  const i = Math.min(4, Math.floor(Math.log(n) / Math.log(1024)));
  return `${(n / 1024 ** i).toLocaleString(undefined, { maximumFractionDigits: i ? 1 : 0 })} ${["B", "KB", "MB", "GB", "TB"][i]}`;
}
