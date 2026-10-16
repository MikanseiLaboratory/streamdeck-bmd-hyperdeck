import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Configuration } from "./Configuration";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Configuration />
  </StrictMode>
);
