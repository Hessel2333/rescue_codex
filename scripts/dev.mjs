import { createServer } from "vite";

const server = await createServer({ configFile: "vite.config.ts" });
await server.listen();
server.printUrls();

async function close() {
  await server.close();
  process.exit(0);
}

process.on("SIGINT", close);
process.on("SIGTERM", close);
