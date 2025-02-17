import {createApp} from "vue";
import App from "./App.vue";
import {listen} from "@tauri-apps/api/event";

const setupEventListener = async () => {
  // Axumからのイベントをリッスン
  await listen<string>("axum_event", (event) => {
    console.log("Received event from Axum:", event.payload);
    // 必要な処理をここに実装
    handleAxumMessage(event.payload);
  });
}

const handleAxumMessage = (message: string) => {
  // 受け取ったメッセージに対する処理
  alert(`Message from Axum: ${message}`);
}

(async () => {
  await setupEventListener();
  createApp(App).mount("#app");
})()
