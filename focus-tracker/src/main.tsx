// File: src/main.tsx

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// 🌟 V4-안전패치: 멀티 창이 각자 독립된 해시 경로를 판독할 수 있도록 리스너를 결합합니다.
const renderApp = () => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
};

// 해시 변경 시 실시간 리렌더링 감지 및 최초 렌더 가동
window.addEventListener("hashchange", renderApp);
renderApp();