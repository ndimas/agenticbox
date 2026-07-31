from contextlib import asynccontextmanager

from fastapi import FastAPI, WebSocket

from agent_runtime.mcp_server import MCPServer
from agent_runtime.tools.terminal import TerminalTool
from agent_runtime.tools.filesystem import FilesystemTool
from agent_runtime.tools.browser import BrowserTool
from agent_runtime.tools.http import HTTPTool


class AgentRuntime:
    def __init__(self):
        self.tools = {
            "terminal": TerminalTool(),
            "filesystem": FilesystemTool(),
            "browser": BrowserTool(),
            "http": HTTPTool(),
        }
        self.mcp = MCPServer(self.tools)

    async def handle_ws(self, ws: WebSocket):
        await self.mcp.handle(ws)


runtime = AgentRuntime()


@asynccontextmanager
async def lifespan(app: FastAPI):
    yield


app = FastAPI(title="Agent Runtime", lifespan=lifespan)


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.get("/tools")
async def list_tools():
    return {
        name: tool.definition()
        for name, tool in runtime.tools.items()
    }


@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    await ws.accept()
    await runtime.handle_ws(ws)


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=9000)
