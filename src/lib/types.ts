export interface FileEntry {
  name: string;
  path: string;
  kind: "file" | "dir" | "symlink";
  size?: number;
  modified?: number;
  permissions?: string;
}

export interface TransferTask {
  id: string;
  kind: "upload" | "download";
  connectionId: string;
  localPath: string;
  remotePath: string;
  fileName: string;
  totalBytes: number;
  transferredBytes: number;
  state: "queued" | "running" | "paused" | "cancelled" | "completed" | "failed";
  speed?: number | null;
  etaSecs?: number | null;
}

export interface Site {
  id: string;
  name: string;
  protocol: "sftp" | "ftp" | "ftps";
  host: string;
  port: number;
  username: string;
  keyPath?: string;
  folder?: string;
  note?: string;
}
