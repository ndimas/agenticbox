from pathlib import Path
from typing import Any, Dict


class FilesystemTool:
    def __init__(self, root: str = "/workspace"):
        self.root = Path(root).resolve()

    def definition(self) -> Dict[str, Any]:
        return {
            "name": "filesystem",
            "description": "Read and write files within the workspace",
            "parameters": {
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "enum": ["read", "write", "list"]},
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                },
                "required": ["operation", "path"],
            },
        }

    def _resolve(self, path: str) -> Path:
        target = (self.root / path).resolve()
        if not str(target).startswith(str(self.root)):
            raise PermissionError("Path outside workspace")
        return target

    async def invoke(self, args: Dict[str, Any]) -> Dict[str, Any]:
        op = args["operation"]
        target = self._resolve(args["path"])
        if op == "read":
            if not target.exists():
                return {"error": "File not found"}
            return {"content": target.read_text(errors="replace")}
        elif op == "write":
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(args.get("content", ""))
            return {"status": "written"}
        elif op == "list":
            if not target.is_dir():
                return {"error": "Not a directory"}
            return {"entries": [str(p.relative_to(self.root)) for p in target.iterdir()]}
        return {"error": "Unknown operation"}
