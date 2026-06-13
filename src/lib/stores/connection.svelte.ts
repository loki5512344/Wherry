import { writable, derived } from "svelte/store";

export interface Connection {
  id: string;
  label: string;
  protocol: "sftp" | "ftp" | "ftps";
  host: string;
  status: "connected" | "disconnected" | "connecting" | "error";
}

function createConnections() {
  const { subscribe, set, update } = writable<Connection[]>([
    // Моковые соединения для разработки UI
    { id: "srv1", label: "srv1", protocol: "sftp", host: "192.168.1.1", status: "connected" },
    { id: "srv2", label: "srv2", protocol: "ftp", host: "192.168.1.2", status: "disconnected" },
    { id: "backup", label: "backup", protocol: "ftps", host: "backup.example.com", status: "disconnected" },
  ]);

  return {
    subscribe,
    add(conn: Connection) {
      update(cs => [...cs, conn]);
    },
    remove(id: string) {
      update(cs => cs.filter(c => c.id !== id));
    },
    setStatus(id: string, status: Connection["status"]) {
      update(cs => cs.map(c => c.id === id ? { ...c, status } : c));
    },
  };
}

export const connections = createConnections();
export const activeId = writable<string | null>("srv1");

export const activeConnection = derived(
  [connections, activeId],
  ([conns, id]) => conns.find(c => c.id === id) ?? null
);
