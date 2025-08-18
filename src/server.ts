import { createServer } from "http";
import { app } from "./app";
import { PORT } from "./config";
import { setupSocket } from "./sockets";

const httpServer = createServer(app);
setupSocket(httpServer);

// lắng nghe toàn mạng LAN
httpServer.listen(PORT, "0.0.0.0", () => {
  console.log(`Server listening on http://0.0.0.0:${PORT}`);
});
