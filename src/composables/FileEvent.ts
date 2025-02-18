import {listen} from "@tauri-apps/api/event";

export const useFileEvent = () => {
  const setupEventListener = async (callback_handler: (payload: string) => void) => {
    // Axumからのイベントをリッスン
    await listen<string>("axum_event", (event) => {
      console.log("Received event from Axum:", event.payload);

      callback_handler(event.payload);
    });
  }

  return {
    setupEventListener
  }
}